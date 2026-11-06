## Design
Remove the profile from the existing connector service, retaining its dependency, isolation and environment configuration. The Cloudflare dashboard must route to http://web:8080. The TCP 7844 path is needed for HTTP/2 fallback; QUIC can operate when UDP 7844 is available.
