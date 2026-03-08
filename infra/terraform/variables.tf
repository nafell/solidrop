variable "aws_region" {
  description = "AWS region for all resources"
  type        = string
  default     = "ap-northeast-1"
}

variable "project_name" {
  description = "Project name used for resource naming"
  type        = string
  default     = "solidrop"
}

variable "bucket_name" {
  description = "S3 bucket name for art storage"
  type        = string
  default     = "nafell-solidrop-storage"
}
