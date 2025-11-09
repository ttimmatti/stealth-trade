anchor build && \
mb-test-validator \
 --reset \
 --bpf-program BTWAqWNBmF2TboMh3fxMJfgR16xGHYD7Kgr2dPwbRPBi scripts/local-dumps/BTWAqWNBmF2TboMh3fxMJfgR16xGHYD7Kgr2dPwbRPBi.so \
 --bpf-program KeyspM2ssCJbqUhQ4k7sveSiY4WjnYsrXkC8oDbwde5 scripts/local-dumps/KeyspM2ssCJbqUhQ4k7sveSiY4WjnYsrXkC8oDbwde5.so \
 --bpf-program 3Qu3rYLyv2BCjkpBGufgnyjtHAvEzgKVR5AHMpgAaGqS target/deploy/private_payments.so
