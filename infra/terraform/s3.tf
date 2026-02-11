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
