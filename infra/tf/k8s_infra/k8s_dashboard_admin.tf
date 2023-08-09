# resource "kubernetes_manifest" "serviceaccount_kubernetes_dashboard_admin_user" {
#   depends_on = 
#   manifest = {
#     "apiVersion" = "v1"
#     "kind" = "ServiceAccount"
#     "metadata" = {
#       "name" = "admin-user"
#       "namespace" = "kubernetes-dashboard"
#     }
#   }
# }

# resource "kubernetes_manifest" "clusterrolebinding_admin_user" {
#   manifest = {
#     "apiVersion" = "rbac.authorization.k8s.io/v1"
#     "kind" = "ClusterRoleBinding"
#     "metadata" = {
#       "name" = "admin-user"
#     }
#     "roleRef" = {
#       "apiGroup" = "rbac.authorization.k8s.io"
#       "kind" = "ClusterRole"
#       "name" = "cluster-admin"
#     }
#     "subjects" = [
#       {
#         "kind" = "ServiceAccount"
#         "name" = "admin-user"
#         "namespace" = "kubernetes-dashboard"
#       },
#     ]
#   }
# }


