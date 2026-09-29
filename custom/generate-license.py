# generate-license.py — 激活码生成工具（服务端/离线使用）
#
# 用法:
#   python generate-license.py --device-id <设备ID> --tier <free|standard|premium> [--expire-days 365]
#
# 依赖:
#   pip install cryptography
#
# 原理:
#   激活码 = base64(json_payload) + "." + base64(hmac_sha256(payload + secret))
#   客户端用相同 secret 验证签名。
#
# 生产环境应改用 RSA 私钥签名 + 客户端内嵌公钥验签。

import argparse
import base64
import hashlib
import hmac
import json
import sys
from datetime import datetime, timedelta, timezone


def generate_license(device_id: str, tier: str, expire_days: int | None, secret: str) -> str:
    payload = {
        "device_id": device_id,
        "tier": tier,
    }

    if expire_days is not None:
        expire_at = (datetime.now(timezone.utc) + timedelta(days=expire_days)).isoformat()
        payload["expire_at"] = expire_at

    payload_json = json.dumps(payload, separators=(",", ":")).encode("utf-8")
    payload_b64 = base64.b64encode(payload_json).decode("ascii")

    signature = hmac.new(secret.encode("utf-8"), payload_b64.encode("utf-8"), hashlib.sha256).digest()
    signature_b64 = base64.b64encode(signature).decode("ascii")

    return f"{payload_b64}.{signature_b64}"


def main():
    parser = argparse.ArgumentParser(description="Generate Handy activation license code")
    parser.add_argument("--device-id", required=True, help="Target device ID")
    parser.add_argument("--tier", required=True, choices=["free", "standard", "premium"], help="License tier")
    parser.add_argument("--expire-days", type=int, default=None, help="Days until expiry (default: permanent)")
    parser.add_argument(
        "--secret",
        default="dev-secret-key-change-in-production",
        help="HMAC secret (must match client's CN_LICENSE_SECRET env var)",
    )

    args = parser.parse_args()

    license_code = generate_license(args.device_id, args.tier, args.expire_days, args.secret)

    print("=== Handy License Code ===")
    print(f"Device ID : {args.device_id}")
    print(f"Tier      : {args.tier}")
    print(f"Expires   : {f'{args.expire_days} days' if args.expire_days else 'permanent'}")
    print(f"\nActivation Code:\n{license_code}")
    print("\n--- Copy the code above and send to the user ---")


if __name__ == "__main__":
    main()