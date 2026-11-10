## ADDED Requirements
### Requirement: 範囲外の重複
重複表示 SHALL 範囲内のURLに一致する記事をライブラリ全体から集計する。
#### Scenario: 境界をまたぐ重複
- **WHEN** 一致する記事が別の範囲にある
- **THEN** 同じグループとして表示し、元記事の状態を維持する。
