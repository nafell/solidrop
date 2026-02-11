resource "aws_iam_user" "api" {
  name = "${var.project_name}-api"
}

resource "aws_iam_user_policy" "api_s3_access" {
  name = "${var.project_name}-s3-access"
  user = aws_iam_user.api.name

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "s3:PutObject",
          "s3:GetObject",
          "s3:DeleteObject",
          "s3:ListBucket",
        ]
        Resource = [
          aws_s3_bucket.art_storage.arn,
          "${aws_s3_bucket.art_storage.arn}/*",
        ]
      }
    ]
  })
}
