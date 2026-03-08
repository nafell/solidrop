# ADR-004: サーバーサイドDB不使用・S3をシングルソースオブトゥルースとする

**ステータス:** Superseded
**決定日:** 2026-02
**Superseded by:** [ADR-006](006-valkey-metadata-index.md)
**関連:** `docs/design/architecture.md §solidrop-api-server`, `README.md §判断5`

---

## Context

初期設計（Phase 0）では、MVPの実装複雑性を最小化するため、
「サーバーサイドDBを持たず、S3 APIのみで一覧・メタデータ参照を完結させる」方針を採用した。

## Decision

本ADRは **ADR-006 により置き換え済み**。
現在は、S3を最終的なデータ実体の保管先としつつ、API層にValkey永続インデックスを持つ。

## Consequences

- 本ADRは履歴保存のため残す。
- 新規実装・運用判断は ADR-006 を参照する。
