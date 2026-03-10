resource "aws_s3_bucket" "art_storage" {
  bucket = var.bucket_name
}

resource "aws_s3_bucket_versioning" "art_storage" {
  bucket = aws_s3_bucket.art_storage.id

  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "art_storage" {
  bucket = aws_s3_bucket.art_storage.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

resource "aws_s3_bucket_public_access_block" "art_storage" {
  bucket = aws_s3_bucket.art_storage.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_cors_configuration" "art_storage" {
  bucket = aws_s3_bucket.art_storage.id

  cors_rule {
    allowed_headers = ["*"]
    allowed_methods = ["GET", "PUT", "HEAD"]
    allowed_origins = [
      "http://localhost:5173",
      "http://127.0.0.1:5173",
      "https://staging.web.solidrop.nafell.dev",
      "https://web.solidrop.nafell.dev",
    ]
    expose_headers  = ["ETag"]
    max_age_seconds = 3600
  }
}

resource "aws_s3_bucket_lifecycle_configuration" "art_storage" {
  bucket = aws_s3_bucket.art_storage.id

  rule {
    id     = "archive-to-glacier"
    status = "Enabled"

    filter {
      prefix = "archived/"
    }

    transition {
      days          = 90
      storage_class = "GLACIER_IR"
    }
  }
}
