# R1 — FQDN egress that survives CDN rotation

**Status: SHIPPED on both sides, with five verified residual gaps.**
The Stage 1 design this document originally proposed landed as **agentcell v0.2.1**
(`fc3102e`, tagged, clean tree) and, on the **cloudcell** side, on `main` (`7508145`
+ `fcc60b2` — validated egress, the `--capabilities` probe, and 502/409 mapping).
What remains is §4, one of which defeats v0.2.1's own "loud failure" guarantee, plus
§5 (the host-side proxy).

**Pins for every claim below:**

```text
agentcell  v0.2.1 = fc3102e (== origin/main, clean)
           commits 5c95cd3, 7138471, 9e02844, fc3102e
           published crate: agentcell-0.2.1 (.crate from static.crates.io, 44141 B)
cloudcell  fcc60b2 (== origin/main when this was written)
```

No SQL is proposed or revised here, so the `verify-design-doc` DDL traps do not
apply. Diagrams are ASCII (as in `docs/architecture.md`); there are no mermaid
blocks to render — `npx @mermaid-js/mermaid-cli` could not install here.

## 1. Requirement

The Zene agent runs this loop inside a cell:

```text
create sandbox(toolchain) → git clone → install deps → build/test/lint → read exit code → destroy
```

R1 asked for a repeatable `--egress host[:port]`, every entry applied, and every A
record of each host allowlisted, because `static.crates.io`,
`files.pythonhosted.org` and `registry.npmjs.org` are CDNs.

## 2. Shipped state

### 2.1 agentcell v0.2.1

| Change | Where |
| --- | --- |
| repeatable `--egress HOST[:PORT]`, port default 443, max 16, rejects empty/IPv6/duplicates | `src/sand.c` (`egress_add`, `--capabilities`) |
| `sand` sends the cell's own nameservers: `NETUP <pid> EGRESS h p … RESOLV <addr> …` | `src/sand.c` `resolv_collect()` — reads `/run/systemd/resolve/resolv.conf` (the file it bind-mounts in), then the rootfs, then `/etc/resolv.conf` |
| daemon resolves each host with `res_nquery` against **those** servers, union of all A records, TTL captured | `src/agentlsm.c` `resolve_egress()` |
| refresh on the record TTL, capped at 60 s, inside the existing 100 ms serve loop (no timerfd) | `src/agentlsm.c` `egress_refresh()` |
| transient refresh failure keeps the last good set | `src/agentlsm.c` |
| loud failure: `ERR egress_unresolved <host>` / `egress_too_many_ips` / `egress_too_many_hosts` / `line_too_long`, veth rolled back | `src/agentlsm.c` |
| `sand` exits nonzero with `egress unavailable: …` instead of falling back to `--net none` **when egress was requested** | `src/sand.c` (`return -2`) |
| non-truncating NETUP builder, `cmd[4096]`, larger control buffers | both |
| `sand --capabilities` → `egress_multi=1 egress_refresh=1 egress_resolv=1 env=1 env_file=1 secret=1 workdir_size=1` | `src/sand.c` |

### 2.2 cloudcell

On `main` (`fcc60b2`) — note that two of these landed in the same push as this doc was
being written, hence the verification section below records them as read from a
working tree rather than from a published revision:

- `egress` is a repeatable `Vec<String>` on create, and the cell path passes the whole
  list to `sand` (PR #1 changes only how the network mode is derived from it).
- `validate_egress()`: rejects empty/whitespace, brackets, IPv6 literals, bad ports;
  dedups; create returns **400 `egress_invalid`**.
- `probe_caps()`: runs `sand --capabilities` and requires `egress_multi=1`,
  `egress_refresh=1`, `egress_resolv=1`, else refuses to run.
- Egress provisioning failure surfaces as **502** instead of a network-less cell, and
  `error.reason = "snapshot_not_packed"` is added to the error body.
- `classify_failure()` maps a nonzero exec onto `oom` / `disk_full` /
  `network_denied` / `nonzero`.
- Warm caches, git credentials and `disk_bytes` are wired through (`caches_json`).
- `docs/sandbox.md` describes the v0.2.1 mechanism (cell nameservers + TTL refresh)
  and the validation probe.

`probe_caps()` inspects the `sand` binary only. §4-G1 is about the *daemon* it will
actually talk to, so that gate does not catch a version skew.

### 2.3 The crate does not ship the daemon

`agentcell-0.2.1.crate` contains `src/sand.c`, `src/libagentcell.c`,
`src/agentcell.h`, `src/arch/*`, `rust/`, `build.rs`, `Cargo.toml` — but **no
`src/agentlsm.c`**; `build.rs` only declares `rerun-if-changed=src/sand.c`. So a
crates.io consumer gets `sand` and the FFI library, and must install the root
daemon out of band. This matters for §4-G2.

## 3. Why the change was needed (kept as the record)

### 3.1 Resolver skew *and* rotation

Two resolvers are in play, and neither was stable:

- `agentlsm` resolved with `getaddrinfo` on the host's `/etc/resolv.conf`
  (systemd-resolved stub `127.0.0.53`).
- The cell used the host's **upstream** list, which `sand` bind-mounts over the
  cell's `/etc/resolv.conf` (`src/sand.c` `do_mounts()`, `up =
  /run/systemd/resolve/resolv.conf`). A rootfs shipping `nameserver 1.1.1.1`
  (`deploy/gcp/pack-rootfs.sh:88`) was overridden on this path.

Four measured views of one name:

```console
$ /tmp/ares2 <ns> static.crates.io          # res_nquery, one server per line
@116.228.111.118  static.crates.io  n=383 ips: 199.232.162.137   # cell upstream #1
@116.228.111.118  static.crates.io  n=383 ips: 146.75.114.137    # …same server, next query
@180.168.255.18   static.crates.io  n=394 ips: 146.75.114.137    # cell upstream #2
@1.1.1.1          static.crates.io  n=184 ips: 151.101.2.137 151.101.66.137 151.101.130.137 151.101.194.137
$ getent ahostsv4 static.crates.io | awk '{print $1}' | sort -u
146.75.114.137                            # what the old agentlsm getaddrinfo() saw
```

The same cell resolver answered `199.232.162.137` on one query and
`146.75.114.137` on the next, so the defect was never only cross-resolver skew —
it was rotation *within* one resolver. A snapshot taken at cell start could not be
complete; `sand --egress static.crates.io` installed a rule for an address the cell
might never dial, and the agent-visible failure was `network_denied` with no hint
that the *allowlist* was resolved elsewhere.

### 3.2 Answer-set size (still relevant for sizing `MAX_EG_IPS`)

```console
$ for h in static.crates.io files.pythonhosted.org registry.npmjs.org; do
      printf "%-26s " "$h"; getent ahostsv4 "$h" | awk '{print $1}' | sort -u | wc -l; done
static.crates.io            1
files.pythonhosted.org      4
registry.npmjs.org         12
$ /tmp/acount <13 realistic CI hosts, including the three above>
TOTAL unique A records: 31            (MAX_EG_IPS = 128 per cell)
```

`registry.npmjs.org` alone consumes 12 of 128 slots. The realistic CI set fits with
headroom, so `egress_too_many_ips` is a guard rail rather than a live constraint —
stated plainly instead of inflated.

### 3.3 The failures that were silent (all now loud, per §2.1)

```console
$ ./sand --egress static.crates.io --egress files.pythonhosted.org:443 --net veth -- /bin/echo hi
sand: --net veth needs the agentlsm daemon (sudo agentlsm serve) — falling back to --net none
hi
sand: exited 0
```

That was v0.2.0: exit 0, no network, stderr only (and cloudcell forwarded that
stderr at `tracing::debug`). The daemon also replied `OK` with zero rules installed
when every host failed to resolve, and discarded an over-long control line.

## 4. Residual gaps (verified against v0.2.1)

Ordered by discovery, not severity — **G5 is the most severe**, because it affects the
default create path rather than a version-skew edge case.

### G1 — `sand` discards the reply counts, so a v0.2.0 daemon still yields a silently network-less cell

**Fix proposed: agentcell PR [#2](https://github.com/EeroEternal/agentcell/pull/2)**
(branch `fix/netup-reply-counts`, CI green) — not yet merged, so v0.2.1 as
released still has this hole.

The daemon now answers with counts:

```c
/* src/agentlsm.c (v0.2.1) */
snprintf(rep, repn, "OK vethc%d 10.200.%u.%u 10.200.%u.%u %d %d\n",
         idx, (b + 2) >> 8, (b + 2) & 255, (b + 1) >> 8, (b + 1) & 255,
         g_nets[slot].n_hosts, g_nets[slot].n_eg);
```

but `sand` validates only the first three fields and throws the rest away:

```c
/* src/sand.c (v0.2.1) */
if (lsm_cmd(cmd, rep, sizeof rep) < 0 || strncmp(rep, "OK ", 3) ||
    sscanf(rep + 3, "%63s %63s %63s",
           C.veth_if, C.veth_ip, C.veth_gw) != 3) {
```

Against a **v0.2.0** daemon, `sand` sends `… RESOLV <addr> EGRESS …`; that daemon
parses after the pid, sees `RESOLV` where it expects `EGRESS`, matches nothing, sets
`n_eg = 0`, installs only the base `DROP` rules, and replies in the old 3-field
format:

```console
$ git show v0.2.0:src/agentlsm.c | grep '"OK vethc'
409:    snprintf(rep, repn, "OK vethc%d 10.200.%u.%u 10.200.%u.%u\n",
```

Three tokens saturate `%63s %63s %63s`, the check passes, and the cell runs with
**zero** egress rules while `sand` and cloudcell both report success. Executed with
v0.2.1's exact format string against both reply shapes:

```console
$ /tmp/sscanf_check              # sscanf(old + 3, "%63s %63s %63s", …)
v0.2.0 reply -> sscanf=3  if=vethc0 cell=10.200.0.2 gw=10.200.0.1  => check (!=3)? ACCEPTED (hole)
v0.2.1 reply -> sscanf=3  if=vethc0 cell=10.200.0.2 gw=10.200.0.1  => check (!=3)? ACCEPTED
```

This is precisely the failure class v0.2.1's changelog claims to have eliminated
("The success reply now carries host/address counts" — carried, but never read).
It also slips past cloudcell's gate, because `probe_caps()` only inspects the
`sand` binary, not the daemon it will talk to.

**Suggested fix (defence in depth, ~4 lines) — implemented in PR #2**, together with a
`tests/stub-agentlsm` stub daemon and four unprivileged regression tests, because the
branch that matters cannot be produced with a real daemon:

```c
int n_hosts = -1, n_ips = -1;
if (lsm_cmd(cmd, rep, sizeof rep) < 0 || strncmp(rep, "OK ", 3) ||
    sscanf(rep + 3, "%63s %63s %63s %d %d",
           C.veth_if, C.veth_ip, C.veth_gw, &n_hosts, &n_ips) != 5 ||
    n_hosts != C.n_egress || n_ips <= 0) {
    /* → "egress unavailable" (return -2), same path as any other ERR */
}
```

(The shape merged in PR #2 keeps the three-field reply acceptable when *no* egress was
requested, so plain `--net veth` is unchanged, and names what the daemon actually did
— `installed 0 of 1 host(s), 1 address(es)` — instead of failing bare.)

### G2 — the daemon has no version or capability surface

```console
$ grep -n "version\|capabilit" src/agentlsm.c
$                       # no output
```

`agentlsm` cannot be probed: there is no `--version`, no `--capabilities`, and no
protocol negotiation over the control socket, while the daemon is absent from the
published crate (§2.3). So the *only* protocol signal available to `sand` is the
reply shape — which G1 shows it does not check. A node can therefore run
`sand` v0.2.1 with `agentlsm` v0.2.0 and pass every gate cloudcell has.

**Suggested fix:** `agentlsm --version` for the binary plus a `CAPS` control command
(same socket, ~10 lines) so `sand` asserts protocol compatibility *before* installing
rules, and cloudcell can gate on the daemon the node will actually use. G1 remains
worth doing anyway: a version string can lie, a count cannot.

### G3 — a `RESOLV`-less `NETUP` silently restores the old skew

`resolve_egress()` falls back to `getaddrinfo` (host stub) when `n_ns == 0`. That
keeps an old `sand` working against a new daemon, but it is exactly the §3.1 bug,
and it fails the *good* way for the wrong reason: the reply is a valid 5-field `OK`
with `n_ips > 0`, so even the G1 fix would not flag it.

**Suggested fix:** reply `ERR egress_no_resolv` (or at minimum log it) when
`n_eg > 0 && n_ns == 0`. The daemon is single-tenant per node, so a loud refusal is
cheaper than a client that silently went deaf.

### G4 — no test covers the control-protocol contract

```console
$ grep -n "capabilities\|egress_unresolved\|n_hosts\|NETUP" tests/run.sh
185:section "egress + capabilities"
186:t_out "capabilities advertises egress_refresh" "egress_refresh=1" \
187:      ./sand --capabilities
```

`--capabilities` text is asserted; the NETUP reply contract is not. End-to-end tests
need root (already gated by `sudook()` at `tests/run.sh:49`), but the *parsing* does
not: extract the reply check into a pure function
(`parse_netup_reply(rep, want_hosts, &n_ips)`) and unit-test it against the v0.2.0
and v0.2.1 reply strings — that is G1's regression test. Note `LSM_SOCK` is a
compile-time constant in both `sand.c` and `agentlsm.c` (no env override), so stub-
daemon tests are not possible without adding one; `tests/run.sh` already isolates
cell sockets through `XDG_RUNTIME_DIR`, so an `AGENTCELL_LSM_SOCK` override would be
consistent with the existing harness.

### G5 — a veth with no allowlist was unrestricted NAT, and cloudcell always asked for one

**Fix proposed: cloudcell PR [#1](https://github.com/EeroEternal/cloudcell/pull/1)**
(branch `fix/no-veth-without-egress`, CI green) — not yet merged.

Found while reviewing the v0.2.1 egress work, and worse than G1: the *default*
create path handed out a sandbox that could reach anything.

`agentlsm` installs the per-cell `DROP` rules only from the `n_eg > 0` branch of
`net_up()` — `egress_base()` is called nowhere else (`src/agentlsm.c:577`, teardown
at `:600`, `:617`, `:635`):

```c
    if (n_eg > 0) {
        egress_base(idx, 1);          /* ← the only place the allowlist is enforced */
        ...
    }
```

Meanwhile `nat_ensure()` installs a **global** `FORWARD -s 10.200.0.0/16 -j ACCEPT`
plus MASQUERADE. With no per-cell rules, a cell falls through to that global `ACCEPT`
and can reach anything it can route; only link-local `169.254/16` is dropped. So
`--net veth` **without** `--egress` is unrestricted NAT, not default-deny.

cloudcell passed `net_veth: true` unconditionally (old `src/sandbox.rs:555`) together
with whatever `egress` list the caller gave, so `POST /api/v1/sandboxes` without an
`egress` field produced a cell with unrestricted outbound access — while
`docs/sandbox.md` promised "loopback-only when no entries are given". The exposure was
the default path, not an opt-in. The "per-sandbox default-deny egress" property in the
original Zene brief therefore held only when at least one entry was passed.

This one is **code-read, not runtime-verified**: confirming it with `iptables -L
FORWARD` needs root, which this workstation does not have (see §7). The call sites are
unambiguous, but it has not been observed on a live node.

**Fix:** derive the network mode from the allowlist (`cell::net_args()`: entries →
`--net veth --egress …`, empty → `--net none`) and drop `SpawnOpts.net_veth` so the
combination cannot be constructed again. `agentlsm` keeps `--net veth` = unrestricted
NAT — that is arguably a legitimate feature (AgentCell's README calls veth "real
networking with NAT") and changing it is a breaking semantic decision; the defect was
cloudcell assuming a veth implied an allowlist.

## 5. Next: the host-side CONNECT proxy (not started)

Still worth doing, now for the R4/R5 milestone rather than for R1:

```text
cell netns (10.200.b.2)
  app -> 10.200.b.2:443            # or http_proxy=http://10.200.b.1:PROXY_PORT
    |
    |  [proxy-oblivious clients only] DNAT 80,443 -> 10.200.b.1:PROXY_PORT
    v
host: vethhN (10.200.b.1)          # FORWARD: allow .2 -> proxy_ip:PROXY_PORT, else DROP
    |
    v
cc-proxy (root, one per node)
  - CONNECT host:port -> allowlist on HOST (no TLS MITM, no SNI)
  - resolves upstream per connection, dials, splices bytes
  - future home of R4 caches and R5 token redaction
```

Why it is still a better home than the TTL-refresh loop: `git`, `cargo`, `pip`,
`npm` and `go` all honour `http_proxy`, and enforcing on the plaintext
`CONNECT host:port` line needs neither MITM nor SNI, makes the allowlist a
hostname set instead of a rotating IP set, and reduces the per-cell firewall rule
set to one static address. `--egress` keeps its meaning of "these hostnames are
reachable".

Proxy-oblivious clients are a separate, optional stage: they need `DNAT` (not
`REDIRECT` — the proxy is in the host netns) plus reading **SNI** from the
ClientHello, and non-TLS/non-HTTP protocols have no hostname to allowlist at all.
That stage needs an explicit `ip:port` escape hatch rather than pretending every
protocol has a hostname.

## 6. Non-goals

- **TLS MITM.** Never: the CONNECT allowlist needs no certificate substitution, and
  the optional SNI stage only reads a plaintext field.
- **Per-connection egress attribution** (which exec dialed which host). The
  `agentlsm` audit stream (`docs/sandbox.md` "Not implemented") would carry it.
- **DNS policy.** `udp/53` to any destination stays allowed, a DNS-exfiltration
  channel. Pre-existing and out of scope; noted so it is not read as covered.
- **Warm caches / git credentials (R4/R5).** Already in place in cloudcell; the
  proxy is where they should eventually converge.

## 7. Verification log

Run on this workstation (Arch, kernel 7.2.3, non-root, `unshare -n` denied):

| Claim | Command |
| --- | --- |
| shipped revisions / clean trees | `git tag --sort=-v:refname`, `git rev-parse --short HEAD`, `git status --porcelain` |
| crate contents and missing daemon | `curl -O https://static.crates.io/crates/agentcell/agentcell-0.2.1.crate; tar tzf`; `grep -n agentlsm build.rs Cargo.toml` |
| resolver skew + rotation (§3.1) | `/tmp/ares2 <ns> <host>` for `116.228.111.118`, `180.168.255.18`, `1.1.1.1` (first server queried twice), plus `getent ahostsv4` |
| answer-set size (§3.2) | `/tmp/acount` over 13 hosts → 31 unique |
| G1 reply contract | `sscanf(rep + 3, "%63s %63s %63s", …) != 3` in v0.2.1 `src/sand.c`; `/tmp/sscanf_check` shows the v0.2.0 reply *passes* that check; `git show v0.2.0:src/agentlsm.c \| grep '"OK vethc'`; fix + tests in PR #2 (`tests/run.sh` 50 passed / 1 failed / 2 skipped vs 45 / 2 / 2 on a pristine v0.2.1 worktree) |
| G1 old-daemon parse of `RESOLV` | `git show v0.2.0:src/agentlsm.c` NETUP branch — matches only `EGRESS` after the pid |
| G2 | `grep -n "version\|capabilit" src/agentlsm.c` → no output |
| G3 | `resolve_egress()` `n_ns == 0` fallback to `getaddrinfo` |
| G4 | `grep -n capabilities tests/run.sh`; `LSM_SOCK` constant in both files |
| shipped capabilities output | `./sand --capabilities` |

**Not verified, and required before §5 is accepted:** every `iptables`/`nft`/DNAT
line in §5. This workstation has no root and no netns (`unshare -n` →
`Operation not permitted`, `sudo` needs a password), so those rules are design, not
tested behavior. The one part of §5.2 executed end to end was the proxy probe:
`example.com` → `CONNECT tunnel failed, response 403`; `static.crates.io/crates/serde/serde-1.0.219.crate`
→ `http_code=200 size=78983`; `registry.npmjs.org/left-pad` → `http_code=200 size=22573`.

## Appendix A — verification probes

Throwaway instruments behind §7, reproduced here because a reviewer re-running this
needs the sources. Not upstream code. Both DNS probes link `-lresolv` and name the
server explicitly, so they show the *cell's* resolver view rather than the host stub's.

```c
/* ares2.c — cc -O1 -o ares2 ares2.c -lresolv; ./ares2 <ns-addr> <host> */
/* prints the A records one resolver returns, with the answer size (§3.1).
 * Run once per nameserver in /run/systemd/resolve/resolv.conf, and twice
 * against the same server to observe the rotation. */
#include <arpa/inet.h>
#include <resolv.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
int main(int c, char **v) { if (c < 3) return 2;
    struct __res_state st; memset(&st, 0, sizeof st);
    if (res_ninit(&st)) return 1;
    st.nscount = 1;
    memset(&st.nsaddr_list[0], 0, sizeof st.nsaddr_list[0]);
    st.nsaddr_list[0].sin_family = AF_INET;
    st.nsaddr_list[0].sin_port = htons(53);
    inet_pton(AF_INET, v[1], &st.nsaddr_list[0].sin_addr);
    unsigned char buf[8192];
    int n = res_nquery(&st, v[2], C_IN, T_A, buf, sizeof buf);
    printf("@%-16s %-24s n=%d ips:", v[1], v[2], n);
    if (n > 0) { ns_msg h; ns_initparse(buf, n, &h);
        for (int i = 0; i < ns_msg_count(h, ns_s_an); i++) { ns_rr rr;
            if (ns_parserr(&h, ns_s_an, i, &rr)) continue;
            if (ns_rr_type(rr) != ns_t_a) continue;
            char ip[64]; inet_ntop(AF_INET, ns_rr_rdata(rr), ip, sizeof ip);
            printf(" %s", ip); } }
    printf("\n"); res_nclose(&st); return 0; }
```

```c
/* acount.c — cc -O1 -o acount acount.c -lresolv; ./acount host... */
/* counts unique A records across hosts (§3.2, MAX_EG_IPS headroom) */
#include <arpa/inet.h>
#include <resolv.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
static int q(const char *host, char ips[][64], int *n) {
    struct __res_state st; memset(&st, 0, sizeof st);
    if (res_ninit(&st)) return -1;
    st.nscount = 1;
    memset(&st.nsaddr_list[0], 0, sizeof st.nsaddr_list[0]);
    st.nsaddr_list[0].sin_family = AF_INET;
    st.nsaddr_list[0].sin_port = htons(53);
    inet_pton(AF_INET, "1.1.1.1", &st.nsaddr_list[0].sin_addr);
    unsigned char buf[8192];
    int r = res_nquery(&st, host, C_IN, T_A, buf, sizeof buf);
    int added = 0;
    if (r > 0) {
        ns_msg h; ns_initparse(buf, r, &h);
        for (int i = 0; i < ns_msg_count(h, ns_s_an); i++) {
            ns_rr rr; if (ns_parserr(&h, ns_s_an, i, &rr)) continue;
            if (ns_rr_type(rr) != ns_t_a) continue;
            char ip[64]; inet_ntop(AF_INET, ns_rr_rdata(rr), ip, sizeof ip);
            int dup = 0;
            for (int j = 0; j < *n; j++) if (!strcmp(ips[j], ip)) { dup = 1; break; }
            if (!dup && *n < 4096) { snprintf(ips[(*n)++], 64, "%s", ip); added++; }
        }
    }
    res_nclose(&st);
    return added;
}
int main(int c, char **v) {
    static char ips[4096][64]; int n = 0;
    for (int i = 1; i < c; i++) printf("%-28s +%d\n", v[i], q(v[i], ips, &n));
    printf("TOTAL unique A records: %d  (MAX_EG_IPS = 128)\n", n);
    return 0;
}
```

```c
/* sscanf_check.c — cc -o sscanf_check sscanf_check.c; proves §4-G1.
 * Uses the format string copied from agentcell v0.2.1 src/sand.c. */
#include <stdio.h>
int main(void) {
    char a[64], b[64], c[64];
    const char *old = "OK vethc0 10.200.0.2 10.200.0.1\n";        /* v0.2.0 */
    const char *neu = "OK vethc0 10.200.0.2 10.200.0.1 1 4\n";    /* v0.2.1 */
    int r1 = sscanf(old + 3, "%63s %63s %63s", a, b, c);
    printf("v0.2.0 reply -> sscanf=%d  if=%s cell=%s gw=%s  => check (!=3)? %s\n",
           r1, a, b, c, r1 != 3 ? "REJECTED" : "ACCEPTED (hole)");
    int r2 = sscanf(neu + 3, "%63s %63s %63s", a, b, c);
    printf("v0.2.1 reply -> sscanf=%d  if=%s cell=%s gw=%s  => check (!=3)? %s\n",
           r2, a, b, c, r2 != 3 ? "REJECTED" : "ACCEPTED");
    return 0;
}
```

```python
# connect_allow.py — python3 connect_allow.py 8899; the §5 core, no root needed.
# Enforces the allowlist on the plaintext CONNECT target: no IP pinning, so
# fastly/cloudflare rotation is irrelevant.
import re, select, socket, sys, threading
ALLOW = {"static.crates.io", "index.crates.io", "crates.io",
         "files.pythonhosted.org", "pypi.org", "registry.npmjs.org"}
PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 8899
def allowed(h, p): return h in ALLOW or any(h.endswith("." + a) for a in ALLOW)
def pump(a, b):
    try:
        while True:
            r, _, _ = select.select([a, b], [], [], 60)
            if not r: break
            for s in r:
                d = s.recv(65536)
                if not d: return
                (b if s is a else a).sendall(d)
    except OSError: pass
    finally:
        for s in (a, b):
            try: s.close()
            except OSError: pass
def handle(c):
    line = c.makefile("rb").readline(8192)
    m = re.match(rb"CONNECT ([^:]+):(\d+) HTTP/1\.[01]", line or b"")
    if not m:
        c.sendall(b"HTTP/1.1 405 Method Not Allowed\r\n\r\n"); c.close(); return
    host, port = m.group(1).decode(), int(m.group(2))
    if not allowed(host, port):
        c.sendall(b"HTTP/1.1 403 Forbidden\r\n\r\n"); c.close(); return
    try: up = socket.create_connection((host, port), 10)
    except OSError:
        c.sendall(b"HTTP/1.1 502 Bad Gateway\r\n\r\n"); c.close(); return
    c.sendall(b"HTTP/1.1 200 Connection established\r\n\r\n")
    pump(c, up)
srv = socket.socket(); srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
srv.bind(("127.0.0.1", PORT)); srv.listen(32)
print(f"listening on 127.0.0.1:{PORT}", flush=True)
while True:
    c, _ = srv.accept()
    threading.Thread(target=handle, args=(c,), daemon=True).start()
```
