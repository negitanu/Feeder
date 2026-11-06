## 1. Configuration and documentation

- [x] 1.1 Document legacy correspondence and dotenv, secret, precedence and restart contracts before implementation.
- [x] 1.2 Add optional provider environment variables and env-token tunnel configuration while preserving the file-token option.
- [x] 1.3 Update configuration validation and operator documentation without exposing credentials.

## 2. Verification

- [x] 2.1 Exercise synthetic env files, both token modes, missing values, precedence, service isolation and Git/build exclusions.
- [x] 2.2 Verify production runtime, browser responses and settings export do not disclose synthetic credentials; retain existing API authorization.
- [x] 2.3 Record Linux/Compose validation and remaining provider/external acceptance; update canonical OpenSpec and validate strictly.
