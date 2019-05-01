// SPDX-License-Identifier: GPL-2.0
#define KBUILD_MODNAME "foo"
#include <linux/bpf.h>
#include "bpf_helpers.h"

struct bpf_map_def SEC("maps") qidconf_map = {
    .type       = BPF_MAP_TYPE_ARRAY,
    .key_size   = sizeof(int),
    .value_size = sizeof(int),
    .max_entries    = 1,
};

struct bpf_map_def SEC("maps") xsks_map = {
    .type = BPF_MAP_TYPE_XSKMAP,
    .key_size = sizeof(int),
    .value_size = sizeof(int),
    .max_entries = 4,
};

SEC("xdp_sock")
int xdp_sock_prog(struct xdp_md *ctx) {
    int *qidconf, key = 0;

    qidconf = bpf_map_lookup_elem(&qidconf_map, &key);
    if (!qidconf)
        return XDP_ABORTED;

    if (*qidconf != ctx->rx_queue_index)
        return XDP_PASS;

    return bpf_redirect_map(&xsks_map, 0, 0);
}

char _license[] SEC("license") = "GPL";
