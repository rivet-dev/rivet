locals {
	# We need to lookup K8s taint effect from the AWS API value
	taint_effects = {
		NO_SCHEDULE        = "NoSchedule"
		NO_EXECUTE         = "NoExecute"
		PREFER_NO_SCHEDULE = "PreferNoSchedule"
	}

	cluster_autoscaler_label_tags = merge([
		for name, group in module.eks.eks_managed_node_groups : {
		for label_name, label_value in coalesce(group.node_group_labels, {}) : "${name}|label|${label_name}" => {
			autoscaling_group = group.node_group_autoscaling_group_names[0],
			key               = "k8s.io/cluster-autoscaler/node-template/label/${label_name}",
			value             = label_value,
		}
	}
	]...)

	cluster_autoscaler_taint_tags = merge([
		for name, group in module.eks.eks_managed_node_groups : {
		for taint in coalesce(group.node_group_taints, []) : "${name}|taint|${taint.key}" => {
			autoscaling_group = group.node_group_autoscaling_group_names[0],
			key               = "k8s.io/cluster-autoscaler/node-template/taint/${taint.key}"
			value             = "${taint.value}:${local.taint_effects[taint.effect]}"
		}
	}
	]...)

	cluster_autoscaler_asg_tags = merge(local.cluster_autoscaler_label_tags, local.cluster_autoscaler_taint_tags)
}

resource "aws_autoscaling_group_tag" "cluster_autoscaler_label_tags" {
	for_each = local.cluster_autoscaler_asg_tags


	autoscaling_group_name = each.value.autoscaling_group

	tag {
		key   = each.value.key
		value = each.value.value

		propagate_at_launch = false
	}
}

