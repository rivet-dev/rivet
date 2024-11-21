# COPIED FROM: https://gist.github.com/tudor/dd17fe94a89b56566738a012d6a3e7a8

#!/usr/bin/env python3

import fdb
import json
from argparse import ArgumentParser
import logging
import signal
import sys
import tempfile
from prometheus_client import (
    start_http_server,
)
from prometheus_client.core import (
    GaugeMetricFamily,
    CounterMetricFamily,
    InfoMetricFamily,
    Metric,
    REGISTRY,
    Sample,
)

logging.basicConfig(format="%(asctime)s %(levelname)s %(message)s")


def str_values(d):
    return {k: str(v) for k, v in d.items()}


BACKUP_STATES = {
    "<undefined>": 0,
    "has errored": 1,
    "has never been started": 2,
    "has been submitted": 3,
    "has been started": 4,
    "is differential": 5,
    "has been completed": 6,
    "has been aborted": 7,
    "has been partially aborted": 8,
}


def backup_state(s):
    return BACKUP_STATES.get(s, 0)


# The standard Prometheus client does not support quantiles in summaries.
# We aim to remedy this, specifically for the FDB quantiles.
# "commit_latency_statistics" : {
#     "count" : 4,
#     "max" : 0.0033512099999999999,
#     "mean" : 0.0030776900000000001,
#     "median" : 0.0030596300000000002,
#     "min" : 0.0027711400000000001,
#     "p25" : 0.0027711400000000001,
#     "p90" : 0.0031287699999999999,
#     "p95" : 0.0031287699999999999,
#     "p99" : 0.0031287699999999999,
#     "p99.9" : 0.0031287699999999999
# }
FDB_QUANTILES = {
    "p25": "0.25",
    "median": "0.5",
    "p90": "0.9",
    "p95": "0.95",
    "p99": "0.99",
    "p99.9": "0.999",
    "max": "1",
}


class FDBSummaryMetricFamily(Metric):
    def __init__(self, name, documentation, *, labels=None):
        Metric.__init__(self, name, documentation, "summary")
        self._labelnames = tuple(labels or [])

    def add_metric(self, labels, value, timestamp=None):
        count = value["count"]
        sum = value["mean"] * count
        label_dict = dict(zip(self._labelnames, labels))

        def add_single(suffix, v):
            d = label_dict.copy()
            self.samples.append(Sample(self.name + suffix, d, v, timestamp))

        def add_quantile(q, v):
            d = label_dict.copy()
            d["quantile"] = q
            self.samples.append(Sample(self.name, d, v, timestamp))

        add_single("_count", count)
        add_single("_sum", sum)
        for k, q in FDB_QUANTILES.items():
            add_quantile(q, value[k])


def get_path(obj, path, prev_path=None):
    prev_path = prev_path or []
    for i, p in enumerate(path):
        if type(obj) != dict:
            log_path = prev_path + path[:i]
            logging.error("Cannot find path {}: not dict".format(".".join(log_path)))
            return None

        obj = obj.get(p)
        if obj is None:
            log_path = prev_path + path[: (i + 1)]
            logging.error("Cannot find path {}".format(".".join(log_path)))
            return None
    return obj


def bool_to_int(v):
    return int(v) if type(v) == "bool" else v


class Info:
    def __init__(self, name, doc, path, *, cb=bool_to_int, cls=GaugeMetricFamily):
        self.name = name
        self.doc = doc
        self.path = path
        self.cb = cb
        self.cls = cls

    def describe(self):
        return self.cls("fdb_" + self.name, self.doc)

    def collect(self, status):
        g = self.describe()
        v = self.cb(get_path(status, self.path))
        if v is not None:
            g.add_metric([], v)
        return g


class LabeledInfo:
    def __init__(
        self,
        name,
        doc,
        labels,
        root_path,
        label_paths,
        value_path,
        *,
        cb=bool_to_int,
        filter=lambda x: True,
        cls=GaugeMetricFamily,
    ):
        self.name = name
        self.doc = doc
        self.labels = labels
        self.root_path = root_path
        self.label_paths = label_paths
        self.value_path = value_path
        self.cb = cb
        self.filter = filter
        self.cls = cls

    def describe(self):
        return self.cls("fdb_" + self.name, self.doc, labels=self.labels)

    def collect(self, status):
        g = self.describe()
        items = get_path(status, self.root_path) or []
        for i, item in enumerate(items):
            if not self.filter(item):
                continue
            prev_path = self.root_path + [str(i)]
            v = self.cb(get_path(item, self.value_path, prev_path))
            if v is not None:
                g.add_metric(
                    [get_path(item, lp, prev_path) or "" for lp in self.label_paths],
                    v,
                )
        return g


class ObjectLabeledInfo:
    def __init__(
        self,
        name,
        doc,
        labels,
        root_path,
        label_paths,
        value_path,
        *,
        use_key_as_first_label=True,
        cb=bool_to_int,
        filter=lambda x: True,
        cls=GaugeMetricFamily,
    ):
        self.name = name
        self.doc = doc
        self.labels = labels
        self.root_path = root_path
        self.label_paths = label_paths
        self.value_path = value_path
        self.use_key_as_first_label = use_key_as_first_label
        self.cb = cb
        self.filter = filter
        self.cls = cls

    def describe(self):
        return self.cls("fdb_" + self.name, self.doc, labels=self.labels)

    def collect(self, status):
        g = self.describe()
        items = get_path(status, self.root_path) or {}
        for key, item in items.items():
            if not self.filter(item):
                continue
            prev_path = self.root_path + [key]
            v = self.cb(get_path(item, self.value_path, prev_path))
            if v is not None:
                label_values = [key] if self.use_key_as_first_label else []
                label_values += (
                    get_path(item, lp, prev_path) or "" for lp in self.label_paths
                )
                g.add_metric(label_values, v)
        return g


class Collector:
    def __init__(self, db, *, status_file=None):
        self.db = db
        self.status_file = status_file
        self.metrics = [
            Info(
                "client_quorum_reachable",
                "Was the proxy able to connect to DB coordinators?",
                ["client", "coordinators", "quorum_reachable"],
            ),
            Info(
                "client_count",
                "Number of connected clients",
                ["cluster", "clients", "count"],
            ),
            Info(
                "cluster_controller_timestamp",
                "Timestamp as reported by cluster controller",
                ["cluster", "cluster_controller_timestamp"],
            ),
            Info(
                "average_partition_size_bytes",
                "Average partition size",
                ["cluster", "data", "average_partition_size_bytes"],
            ),
            Info(
                "moving_data_highest_priority",
                "Moving data: highest priority (task count?)",
                ["cluster", "data", "moving_data", "highest_priority"],
            ),
            Info(
                "moving_data_in_flight_bytes",
                "Moving data: bytes in flight",
                ["cluster", "data", "moving_data", "in_flight_bytes"],
            ),
            Info(
                "moving_data_in_queue_bytes",
                "Moving data: bytes in queue",
                ["cluster", "data", "moving_data", "in_queue_bytes"],
            ),
            Info(
                "moving_data_total_written_bytes",
                "Moving data: total bytes written",
                ["cluster", "data", "moving_data", "total_written_bytes"],
            ),
            Info(
                "partition_count",
                "Partition count",
                ["cluster", "data", "partitions_count"],
            ),
            Info(
                "total_kv_size_bytes",
                "Total KV size",
                ["cluster", "data", "total_kv_size_bytes"],
            ),
            Info(
                "total_disk_used_bytes",
                "Total disk usage",
                ["cluster", "data", "total_disk_used_bytes"],
            ),
            Info(
                "least_operating_space_bytes_log_server",
                "Minimum operating space among all log server processes",
                ["cluster", "data", "least_operating_space_bytes_log_server"],
            ),
            Info(
                "least_operating_space_bytes_storage_server",
                "Minimum operating space among all storage server processes",
                [
                    "cluster",
                    "data",
                    "least_operating_space_bytes_storage_server",
                ],
            ),
            Info(
                "database_available",
                "Is the database available?",
                ["cluster", "database_available"],
            ),
            Info(
                "degraded_process_count",
                "Number of degraded processes",
                ["cluster", "degraded_processes"],
            ),
            Info(
                "full_replication",
                "Is the data fully replicated?",
                ["cluster", "full_replication"],
            ),
            Info(
                "generation",
                "Cluster generation",
                ["cluster", "generation"],
            ),
            Info(
                "incompatible_connection_count",
                "Number of incompatible connections",
                ["cluster", "incompatible_connections"],
                cb=len,
            ),
            ObjectLabeledInfo(
                "machine_cpu_utilization",
                "CPU utilization by machine",
                ["id", "address"],
                ["cluster", "machines"],
                [["address"]],
                ["cpu", "logical_core_utilization"],
            ),
            ObjectLabeledInfo(
                "machine_memory_committed_bytes",
                "Amount of committed (in use) memory by machine",
                ["id", "address"],
                ["cluster", "machines"],
                [["address"]],
                ["memory", "committed_bytes"],
            ),
            ObjectLabeledInfo(
                "machine_memory_free_bytes",
                "Amount of free memory by machine",
                ["id", "address"],
                ["cluster", "machines"],
                [["address"]],
                ["memory", "free_bytes"],
            ),
            Info(
                "page_cache_log_hit_rate",
                "Log hit rate",
                ["cluster", "page_cache", "log_hit_rate"],
            ),
            Info(
                "page_cache_storage_hit_rate",
                "Storage hit rate",
                ["cluster", "page_cache", "storage_hit_rate"],
            ),
            ObjectLabeledInfo(
                "process_cpu_utilization",
                "CPU utilization",
                ["id", "address", "class_type"],
                ["cluster", "processes"],
                [["address"], ["class_type"]],
                ["cpu", "usage_cores"],
            ),
            ObjectLabeledInfo(
                "disk_free_bytes",
                "Amount of free disk space",
                ["id", "address", "class_type"],
                ["cluster", "processes"],
                [["address"], ["class_type"]],
                ["disk", "free_bytes"],
            ),
            ObjectLabeledInfo(
                "disk_total_bytes",
                "Amount of total disk space",
                ["id", "address", "class_type"],
                ["cluster", "processes"],
                [["address"], ["class_type"]],
                ["disk", "total_bytes"],
            ),
            ObjectLabeledInfo(
                "disk_busy",
                "Disk busyness",
                ["id", "address", "class_type"],
                ["cluster", "processes"],
                [["address"], ["class_type"]],
                ["disk", "busy"],
            ),
            ObjectLabeledInfo(
                "disk_read_count",
                "Number of disk reads",
                ["id", "address", "class_type"],
                ["cluster", "processes"],
                [["address"], ["class_type"]],
                ["disk", "reads", "counter"],
                cls=CounterMetricFamily,
            ),
            ObjectLabeledInfo(
                "disk_write_count",
                "Number of disk writes",
                ["id", "address", "class_type"],
                ["cluster", "processes"],
                [["address"], ["class_type"]],
                ["disk", "writes", "counter"],
                cls=CounterMetricFamily,
            ),
            ObjectLabeledInfo(
                "memory_available_bytes",
                "Available memory",
                ["id", "address", "class_type"],
                ["cluster", "processes"],
                [["address"], ["class_type"]],
                ["memory", "available_bytes"],
            ),
            ObjectLabeledInfo(
                "memory_limit_bytes",
                "Memory limit",
                ["id", "address", "class_type"],
                ["cluster", "processes"],
                [["address"], ["class_type"]],
                ["memory", "limit_bytes"],
            ),
            ObjectLabeledInfo(
                "unused_allocated_memory_bytes",
                "Unused allocated memory",
                ["id", "address", "class_type"],
                ["cluster", "processes"],
                [["address"], ["class_type"]],
                ["memory", "unused_allocated_memory"],
            ),
            ObjectLabeledInfo(
                "memory_used_bytes",
                "Used memory",
                ["id", "address", "class_type"],
                ["cluster", "processes"],
                [["address"], ["class_type"]],
                ["memory", "used_bytes"],
            ),
            ObjectLabeledInfo(
                "run_loop_busy",
                "Used memory",
                ["id", "address", "class_type"],
                ["cluster", "processes"],
                [["address"], ["class_type"]],
                ["run_loop_busy"],
            ),
            Info(
                "batch_performance_limited_by",
                "Reason limiting performance for batch transactions",
                ["cluster", "qos", "batch_performance_limited_by"],
                cls=InfoMetricFamily,
                cb=str_values,
            ),
            Info(
                "performance_limited_by",
                "Reason limiting performance for batch transactions",
                ["cluster", "qos", "performance_limited_by"],
                cls=InfoMetricFamily,
                cb=str_values,
            ),
            Info(
                "recovery_state",
                "Recovery state",
                ["cluster", "recovery_state"],
                cls=InfoMetricFamily,
                cb=str_values,
            ),
            Info(
                "workload_bytes_read",
                "Bytes read",
                ["cluster", "workload", "bytes", "read", "counter"],
                cls=CounterMetricFamily,
            ),
            Info(
                "workload_bytes_written",
                "Bytes written",
                ["cluster", "workload", "bytes", "written", "counter"],
                cls=CounterMetricFamily,
            ),
            ObjectLabeledInfo(
                "workload_op_count",
                "Op count",
                ["op"],
                ["cluster", "workload", "operations"],
                [],
                ["counter"],
                cls=CounterMetricFamily,
            ),
            ObjectLabeledInfo(
                "workload_transaction_count",
                "Transaction count",
                ["state"],
                ["cluster", "workload", "transactions"],
                [],
                ["counter"],
                cls=CounterMetricFamily,
            ),
        ]

        roles = [
            "coordinator",
            "proxy",
            "log",
            "storage",
            "cluster_controller",
            "ratekeeper",
            "data_distributor",
            "resolver",
            "master",
        ]

        for r in roles:
            self.metrics.append(
                ObjectLabeledInfo(
                    "is_" + r,
                    "Is this process a {}?".format(r),
                    ["id", "address", "class_type"],
                    ["cluster", "processes"],
                    [["address"], ["class_type"]],
                    ["roles"],
                    cb=lambda roles, r=r: int(any(x["role"] == r for x in roles)),
                )
            )

        self.by_role = {r: [] for r in roles}

        self.by_role["proxy"] += [
            LabeledInfo(
                "proxy_commit_latency",
                "Commit latency statistics",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["commit_latency_statistics"],
                cls=FDBSummaryMetricFamily,
            ),
            # TODO(tudor): support multiple txn classes, not just "default"
            LabeledInfo(
                "proxy_grv_latency",
                "GetReadVersion latency statistics",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["grv_latency_statistics", "default"],
                cls=FDBSummaryMetricFamily,
            ),
        ]

        self.by_role["log"] += [
            LabeledInfo(
                "log_data_version",
                "Data version",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["data_version"],
            ),
            LabeledInfo(
                "log_durable_bytes",
                "Amount of data stored to durable storage (cumulative)",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["durable_bytes", "counter"],
                cls=CounterMetricFamily,
            ),
            LabeledInfo(
                "log_input_bytes",
                "Amount of data received (cumulative)",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["input_bytes", "counter"],
                cls=CounterMetricFamily,
            ),
            LabeledInfo(
                "log_kvstore_available_bytes",
                "Amount of available kvstore bytes",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["kvstore_available_bytes"],
            ),
            LabeledInfo(
                "log_kvstore_free_bytes",
                "Amount of free kvstore bytes",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["kvstore_free_bytes"],
            ),
            LabeledInfo(
                "log_kvstore_total_bytes",
                "Amount of total kvstore bytes",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["kvstore_total_bytes"],
            ),
            LabeledInfo(
                "log_kvstore_used_bytes",
                "Amount of used kvstore bytes",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["kvstore_used_bytes"],
            ),
            LabeledInfo(
                "log_queue_disk_available_bytes",
                "Amount of available disk bytes for queue",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["queue_disk_available_bytes"],
            ),
            LabeledInfo(
                "log_queue_disk_free_bytes",
                "Amount of free disk bytes for queue",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["queue_disk_free_bytes"],
            ),
            LabeledInfo(
                "log_queue_disk_total_bytes",
                "Amount of total disk bytes for queue",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["queue_disk_total_bytes"],
            ),
            LabeledInfo(
                "log_queue_disk_used_bytes",
                "Amount of used disk bytes for queue",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["queue_disk_used_bytes"],
            ),
        ]

        self.by_role["storage"] += [
            LabeledInfo(
                "storage_queried_bytes",
                "Amount of bytes queried (cumulative)",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["bytes_queried", "counter"],
                cls=CounterMetricFamily,
            ),
            LabeledInfo(
                "storage_data_lag_seconds",
                "Data lag (seconds)",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["data_lag", "seconds"],
            ),
            LabeledInfo(
                "storage_data_lag_versions",
                "Data lag (versions)",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["data_lag", "versions"],
            ),
            LabeledInfo(
                "storage_data_version",
                "Highest version",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["data_version"],
            ),
            LabeledInfo(
                "storage_durability_lag_seconds",
                "Durability lag (seconds)",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["durability_lag", "seconds"],
            ),
            LabeledInfo(
                "storage_durability_lag_versions",
                "Durability lag (versions)",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["durability_lag", "versions"],
            ),
            LabeledInfo(
                "storage_finished_queries",
                "Number of finished queries (cumulative)",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["finished_queries", "counter"],
                cls=CounterMetricFamily,
            ),
            LabeledInfo(
                "storage_input_bytes",
                "Amount of input bytes (cumulative)",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["input_bytes", "counter"],
                cls=CounterMetricFamily,
            ),
            LabeledInfo(
                "storage_durable_bytes",
                "Amount of durable bytes (cumulative)",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["durable_bytes", "counter"],
                cls=CounterMetricFamily,
            ),
            LabeledInfo(
                "storage_keys_queried",
                "Number of keys queried (cumulative)",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["keys_queried", "counter"],
                cls=CounterMetricFamily,
            ),
            LabeledInfo(
                "storage_kvstore_available_bytes",
                "Amount of available kvstore bytes",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["kvstore_available_bytes"],
            ),
            LabeledInfo(
                "storage_kvstore_free_bytes",
                "Amount of free kvstore bytes",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["kvstore_free_bytes"],
            ),
            LabeledInfo(
                "storage_kvstore_total_bytes",
                "Amount of total kvstore bytes",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["kvstore_total_bytes"],
            ),
            LabeledInfo(
                "storage_kvstore_used_bytes",
                "Amount of used kvstore bytes",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["kvstore_used_bytes"],
            ),
            LabeledInfo(
                "storage_local_rate",
                "Local rate",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["local_rate"],
            ),
            LabeledInfo(
                "storage_low_priority_queries",
                "Number of low priority queries (cumulative)",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["low_priority_queries", "counter"],
                cls=CounterMetricFamily,
            ),
            LabeledInfo(
                "storage_total_queries",
                "Number of queries (cumulative)",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["total_queries", "counter"],
                cls=CounterMetricFamily,
            ),
            LabeledInfo(
                "storage_mutation_bytes",
                "Amount of bytes mutated (cumulative)",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["mutation_bytes", "counter"],
                cls=CounterMetricFamily,
            ),
            LabeledInfo(
                "storage_mutations",
                "Number of mutations (cumulative)",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["mutations", "counter"],
                cls=CounterMetricFamily,
            ),
            LabeledInfo(
                "storage_query_queue_max",
                "Number of queries in queue (max)",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["query_queue_max"],
            ),
            LabeledInfo(
                "storage_read_latency",
                "Read latency statistics",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["read_latency_statistics"],
                cls=FDBSummaryMetricFamily,
            ),
            LabeledInfo(
                "storage_log_fetch_count",
                "Storage log fetches",
                ["process", "id"],
                [],
                [["process"], ["id"]],
                ["fetches_from_logs", "counter"],
                cls=CounterMetricFamily,
            ),
        ]

        layers = ["backup"]
        self.by_layer = {l: [] for l in layers}

        self.by_layer["backup"] += [
            ObjectLabeledInfo(
                "backup_instance_bytes_sent",
                "Bytes sent by backup agent instance",
                ["id"],
                ["instances"],
                [],
                ["blob_stats", "total", "bytes_sent"],
                cls=CounterMetricFamily,
            ),
            ObjectLabeledInfo(
                "backup_instance_requests_failed",
                "Requests failed by backup agent instance",
                ["id"],
                ["instances"],
                [],
                ["blob_stats", "total", "requests_failed"],
                cls=CounterMetricFamily,
            ),
            ObjectLabeledInfo(
                "backup_instance_requests_successful",
                "Requests successful by backup agent instance",
                ["id"],
                ["instances"],
                [],
                ["blob_stats", "total", "requests_successful"],
                cls=CounterMetricFamily,
            ),
            ObjectLabeledInfo(
                "backup_instance_memory_usage_bytes",
                "Memory usage by backup agent instance",
                ["id"],
                ["instances"],
                [],
                ["memory_usage"],
            ),
            ObjectLabeledInfo(
                "backup_state",
                "Backup state",
                ["tag"],
                ["tags"],
                [],
                ["current_status"],
                cb=backup_state,
            ),
            ObjectLabeledInfo(
                "backup_running",
                "Is backup running?",
                ["tag"],
                ["tags"],
                [],
                ["running_backup"],
            ),
            ObjectLabeledInfo(
                "backup_restorable",
                "Is backup restorable?",
                ["tag"],
                ["tags"],
                [],
                ["running_backup_is_restorable"],
            ),
            ObjectLabeledInfo(
                "backup_restorable_lag",
                "Lag from last restorable timestamp",
                ["tag"],
                ["tags"],
                [],
                ["last_restorable_seconds_behind"],
            ),
        ]

    def describe(self):
        for g in self.metrics:
            yield g.describe()
        for r in self.by_role.values():
            for g in r:
                yield g.describe()
        for r in self.by_layer.values():
            for g in r:
                yield g.describe()

    def collect(self):
        if self.status_file:
            with open(self.status_file, "rb") as f:
                status_blob = f.read()
        else:
            try:
                status_blob = self.db[b"\xff\xff/status/json"]
            except Exception:
                logging.exception("Error retrieving status from DB")
                return

        status = json.loads(status_blob)
        if not status["client"]["database_status"]["available"]:
            messages = "; ".join(m["description"] for m in status["client"]["messages"])
            logging.error(
                "DB not available when retrieving status: {}".format(messages)
            )

        for g in self.metrics:
            try:
                yield g.collect(status)
            except Exception:
                logging.exception("Error when collecting metric {}".format(g.name))

        by_role = dict((k, []) for k in self.by_role.keys())
        for pid, proc in status.get("cluster", {}).get("processes", {}).items():
            for role in proc["roles"]:
                role_name = role["role"]
                if not role_name in self.by_role:
                    continue
                role["process"] = pid
                by_role[role_name].append(role)

        for k, metrics in self.by_role.items():
            for g in metrics:
                try:
                    yield g.collect(by_role[k])
                except Exception:
                    logging.exception(
                        "Error when collecting by-role metric {}".format(g.name)
                    )

        for k, metrics in self.by_layer.items():
            layer = status.get("cluster", {}).get("layers", {}).get(k)
            if layer is None:
                continue
            for g in metrics:
                try:
                    yield g.collect(layer)
                except Exception:
                    logging.exception(
                        "Error when connecting by-layer metric {}, layer {}",
                        format(g.name, k),
                    )


def main():
    ap = ArgumentParser()
    ap.add_argument("--prometheus-port", type=int, default=9161, help="Prometheus port")
    ap.add_argument("--fdb-cluster-file", help="FDB cluster file")
    ap.add_argument("--status-file", help="Use status from local file")
    ap.add_argument(
        "--copy-cluster-file",
        help="Copy cluster file to temp location",
        action="store_true",
    )
    args = ap.parse_args()

    cluster_file_path = args.fdb_cluster_file
    if args.copy_cluster_file:
        with open(cluster_file_path, "rb") as old_file:
            content = old_file.read()
            with tempfile.NamedTemporaryFile(prefix="fdb_cluster", delete=False) as f:
                f.write(content)
                cluster_file_path = f.name

    db = None
    status_file = None
    if args.status_file:
        status_file = args.status_file
    else:
        assert args.fdb_cluster_file
        fdb.api_version(730)
        try:
            db = fdb.open(cluster_file=cluster_file_path)
        except Exception:
            logging.exception(
                "Error when connecting to DB, cluster file path = {}".format(
                    cluster_file_path
                )
            )
            sys.exit(1)

    REGISTRY.register(Collector(db, status_file=status_file))
    start_http_server(args.prometheus_port)
    while True:
        signal.pause()


if __name__ == "__main__":
    main()
