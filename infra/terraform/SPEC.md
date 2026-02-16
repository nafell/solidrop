# Infrastructure (Terraform) — Specification

Terraform configurations for provisioning AWS resources. The VPS itself is not managed by Terraform (existing contract, manually administered).

## Managed Resources

| Resource | Terraform ID | File |
|---|---|---|
| S3 bucket | `aws_s3_bucket.art_storage` | `s3.tf` |
| S3 versioning | `aws_s3_bucket_versioning.art_storage` | `s3.tf` |
| S3 encryption (SSE-S3) | `aws_s3_bucket_server_side_encryption_configuration.art_storage` | `s3.tf` |
| S3 public access block | `aws_s3_bucket_public_access_block.art_storage` | `s3.tf` |
| S3 lifecycle (Glacier transition) | `aws_s3_bucket_lifecycle_configuration.art_storage` | `s3.tf` |
| IAM user | `aws_iam_user.api` | `iam.tf` |
| IAM inline policy | `aws_iam_user_policy.api_s3_access` | `iam.tf` |

## Input Variables

| Variable | Type | Default | Required | Notes |
|---|---|---|---|---|
| `aws_region` | string | `ap-northeast-1` | No | Tokyo region — THOUGHT-THROUGH (README §8.1) |
| `project_name` | string | `solidrop` | No | Used for IAM user naming |
| `bucket_name` | string | — | **Yes** | No default — README TBD-1 |

**Decision: bucket_name has no default — deliberate.** The README lists the actual bucket name as TBD-1. The variable forces an explicit choice at `terraform plan/apply` time.

## S3 Bucket Configuration

### Versioning — THOUGHT-THROUGH

**Decision:** Enabled from Phase 1.

**Rationale (README 2.2 §6, §13.3):** Low storage cost overhead for personal use. Preserves file history for future version management UI (Phase 2). "Data preservation now, UI later" approach. The reviewer (Claude) explicitly flagged that backup systems derive value from restore capability, and versioning enables that.

### Server-Side Encryption — THOUGHT-THROUGH

**Decision:** SSE-S3 (AES256, AWS-managed keys).

**Rationale (README §4 NF-3, §9.1):** Files are already client-side encrypted (AES-256-GCM). SSE-S3 adds a second layer at rest. SSE-KMS was not chosen because the per-request cost is unnecessary when client-side encryption is the primary protection.

### Public Access Block — THOUGHT-THROUGH

**Decision:** All four public access block settings enabled.

**Rationale:** Personal data bucket. No public access is ever needed. This is a security baseline, not a design trade-off.

### Lifecycle Policy — THOUGHT-THROUGH

**Decision:** Objects under `archived/` prefix transition to Glacier Instant Retrieval after 90 days.

**Rationale (README §8.1):** Archived files are accessed infrequently but need to remain retrievable within milliseconds (not minutes/hours). Glacier Instant Retrieval provides ~68% cost reduction over S3 Standard while maintaining low-latency access. The 90-day threshold is specified in the README.

**Note:** Only the `archived/` prefix is affected. `active/` and `transfer/` remain in S3 Standard.

## IAM Configuration

### User and Policy — THOUGHT-THROUGH

**Decision:** Dedicated IAM user with an inline policy scoped to the specific S3 bucket.

**Rationale (README §8.1, 2.2 §8):** Minimal privilege principle. The API server on the VPS needs IAM credentials; using a dedicated user with bucket-scoped permissions limits blast radius if the VPS is compromised. Access keys are stored as environment variables on the VPS.

**Allowed S3 actions:**
- `s3:PutObject` — upload via presigned URL
- `s3:GetObject` — download via presigned URL
- `s3:DeleteObject` — file deletion
- `s3:ListBucket` — file listing

**Note:** `CopyObject` (needed for move operations) is not listed as a separate IAM action because S3 CopyObject is authorized through the combination of `GetObject` (source) and `PutObject` (destination) permissions on the same bucket.

**Policy scope:** Both the bucket ARN (`arn:aws:s3:::bucket`) for `ListBucket` and the objects ARN (`arn:aws:s3:::bucket/*`) for object-level operations.

### Security Considerations

**Risk (README RISK-2):** IAM access keys on the VPS are a compromise vector. Mitigations:
- Minimal permissions (no `s3:*`, no IAM management)
- Key rotation (TBD-4, recommended 90-day cycle)
- VPS hardening (SSH key auth only, UFW)

**Migration path (README §15.3):** If the API server moves to Fargate/Lambda, IAM roles replace access keys, eliminating this risk class entirely.

## Not Managed by Terraform

| Resource | Reason |
|---|---|
| XServer VPS | Existing contract, manually provisioned |
| TLS certificates | Let's Encrypt / Caddy on VPS (TBD-2) |
| DNS | Domain configuration (TBD-2) |
| Docker images | Built and deployed manually or via CI/CD (not yet set up) |

## Usage

```bash
cd infra/terraform/

# First time
terraform init

# Preview changes
terraform plan -var="bucket_name=my-solidrop-bucket"

# Apply
terraform apply -var="bucket_name=my-solidrop-bucket"
```

A `.tfvars` file is recommended for repeated use but is gitignored for security.
