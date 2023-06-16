mine_functions:
  ip_addrs_public:
    - mine_function: network.ip_addrs
    - type: public
  ip_addrs_private:
    - mine_function: network.ip_addrs
    - type: private
  nebula_ipv4:
    - mine_function: grains.get
    - key: nebula:ipv4
