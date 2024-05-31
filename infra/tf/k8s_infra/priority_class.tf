# Used for services that can be preempted often.
#
# We set almost everything to this class (even crucial infrastrcuture) because
# we need to wait for Karpenter to boot a new node instead of shutting down
# existing services.
resource "kubernetes_priority_class" "service_priority" {

	metadata {
		name = "service-priority"
	}
	value = 50
}

# Used for anything required to monitor the other services. These should take
# priority no matter what in order to ensure we have visibility on what's going
# on.
resource "kubernetes_priority_class" "monitoring_priority" {

	metadata {
		name = "monitoring-priority"
	}
	value = 50
}

# Used for anything stateful that should not be frequently preempted.
resource "kubernetes_priority_class" "stateful_priority" {

	metadata {
		name = "stateful-priority"
	}
	value = 60
}

# Used for daemons that run on the machines and need to be scheduled no matter what.
resource "kubernetes_priority_class" "daemon_priority" {

	metadata {
		name = "daemon-priority"
	}
	value = 90
}
