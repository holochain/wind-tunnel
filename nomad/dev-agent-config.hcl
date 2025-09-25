data_dir = "/tmp/nomad/data"

plugin "raw_exec" {
  config {
    enabled = true
  }
}

server {
  enabled          = true
  bootstrap_expect = 1
}

client {
  enabled = true
  artifact {
    disable_filesystem_isolation = true
  }
}
