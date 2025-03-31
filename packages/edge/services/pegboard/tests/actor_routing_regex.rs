#[cfg(test)]
mod tests {
    use regex::Regex;
    use rivet_pegboard::util::{build_actor_hostname_and_path_regex, build_actor_hostname_and_path};
    use uuid::Uuid;
    use cluster::types::GuardPublicHostname;
    use rivet_pegboard::types::{EndpointType, GameGuardProtocol};

    #[test]
    fn test_hostname_routing_regex() {
        // Test DNS parent with hostname routing
        let dns_parent = "example.com".to_string();
        let (hostname_regex, path_regex_opt) = build_actor_hostname_and_path_regex(
            EndpointType::Hostname,
            &GuardPublicHostname::DnsParent(dns_parent.clone())
        ).unwrap();

        // Verify that path_regex is None for hostname routing
        assert!(path_regex_opt.is_none());

        // Test valid hostname format
        let actor_id = "11111111-1111-1111-1111-111111111111";
        let port_name = "web";
        let test_hostname = format!("{}-{}.actor.example.com", actor_id, port_name);
        
        let captures = hostname_regex.captures(&test_hostname).unwrap();
        assert_eq!(captures.name("actor_id").unwrap().as_str(), actor_id);
        assert_eq!(captures.name("port_name").unwrap().as_str(), port_name);

        // Test invalid hostnames
        let invalid_hostnames = vec![
            "not-a-uuid-web.actor.example.com",
            "11111111-1111-1111-1111-111111111111.actor.example.com", // missing port name
            "11111111-1111-1111-1111-111111111111-web.wrong.example.com", // wrong subdomain
            "actor.example.com", // missing actor id and port
        ];

        for hostname in invalid_hostnames {
            assert!(hostname_regex.captures(hostname).is_none(), 
                "Should not match invalid hostname: {}", hostname);
        }
    }

    #[test]
    fn test_path_routing_regex() {
        // Test DNS parent with path routing
        let dns_parent = "example.com".to_string();
        let (hostname_regex, path_regex_opt) = build_actor_hostname_and_path_regex(
            EndpointType::Path,
            &GuardPublicHostname::DnsParent(dns_parent.clone())
        ).unwrap();

        // Verify that path_regex is Some for path routing
        assert!(path_regex_opt.is_some());
        let path_regex = path_regex_opt.unwrap();

        // Test hostname is route.actor.example.com
        assert!(hostname_regex.is_match("route.actor.example.com"));
        assert!(!hostname_regex.is_match("invalid.example.com"));

        // Test valid path format
        let actor_id = "22222222-2222-2222-2222-222222222222";
        let port_name = "api";
        
        // Test basic path
        let test_path = format!("/{}-{}", actor_id, port_name);
        let captures = path_regex.captures(&test_path).unwrap();
        assert_eq!(captures.name("actor_id").unwrap().as_str(), actor_id);
        assert_eq!(captures.name("port_name").unwrap().as_str(), port_name);

        // Test path with additional segments
        let test_path_with_segments = format!("/{}-{}/additional/path", actor_id, port_name);
        let captures = path_regex.captures(&test_path_with_segments).unwrap();
        assert_eq!(captures.name("actor_id").unwrap().as_str(), actor_id);
        assert_eq!(captures.name("port_name").unwrap().as_str(), port_name);

        // Test invalid paths
        let invalid_paths = vec![
            "/not-a-uuid-api",
            "/22222222-2222-2222-2222-222222222222", // missing port name
            "22222222-2222-2222-2222-222222222222-api", // missing leading slash
        ];

        for path in invalid_paths {
            assert!(path_regex.captures(path).is_none(), 
                "Should not match invalid path: {}", path);
        }
    }

    #[test]
    fn test_static_hostname_with_path_routing() {
        // Test static hostname with path routing
        let static_hostname = "static.example.org".to_string();
        let (hostname_regex, path_regex_opt) = build_actor_hostname_and_path_regex(
            EndpointType::Path,
            &GuardPublicHostname::Static(static_hostname.clone())
        ).unwrap();

        // Verify path_regex is Some for path routing
        assert!(path_regex_opt.is_some());
        let path_regex = path_regex_opt.unwrap();

        // Test hostname matches exactly
        assert!(hostname_regex.is_match(&static_hostname));
        assert!(!hostname_regex.is_match("other.example.org"));

        // Test valid path format
        let actor_id = "33333333-3333-3333-3333-333333333333";
        let port_name = "game";
        let test_path = format!("/{}-{}/game/session", actor_id, port_name);
        
        let captures = path_regex.captures(&test_path).unwrap();
        assert_eq!(captures.name("actor_id").unwrap().as_str(), actor_id);
        assert_eq!(captures.name("port_name").unwrap().as_str(), port_name);
    }

    #[test]
    fn test_static_hostname_with_hostname_routing_fails() {
        // Test static hostname with hostname routing (should fail)
        let static_hostname = "static.example.net".to_string();
        let result = build_actor_hostname_and_path_regex(
            EndpointType::Hostname,
            &GuardPublicHostname::Static(static_hostname)
        );

        // This combination should return an error
        assert!(result.is_err());
    }

    #[test]
    fn test_hostname_from_build_actor_hostname_with_regex() {
        // Test that hostname generated by build_actor_hostname_and_path is matched by the regex
        let dns_parent = "example.com".to_string();
        let actor_id = Uuid::parse_str("44444444-4444-4444-4444-444444444444").unwrap();
        let port_name = "http";
        let protocol = GameGuardProtocol::Http;

        // Build hostname using build_actor_hostname_and_path
        let (hostname, path) = build_actor_hostname_and_path(
            actor_id,
            port_name,
            protocol,
            EndpointType::Hostname,
            &GuardPublicHostname::DnsParent(dns_parent.clone())
        ).unwrap();

        // Build regex using build_actor_hostname_and_path_regex
        let (hostname_regex, path_regex_opt) = build_actor_hostname_and_path_regex(
            EndpointType::Hostname,
            &GuardPublicHostname::DnsParent(dns_parent)
        ).unwrap();

        // Assert that path is None
        assert!(path.is_none());
        assert!(path_regex_opt.is_none());

        // Match hostname against regex
        let captures = hostname_regex.captures(&hostname).unwrap();
        assert_eq!(captures.name("actor_id").unwrap().as_str(), "44444444-4444-4444-4444-444444444444");
        assert_eq!(captures.name("port_name").unwrap().as_str(), port_name);
    }

    #[test]
    fn test_path_from_build_actor_hostname_with_regex() {
        // Test that hostname and path generated by build_actor_hostname_and_path is matched by the regex
        let dns_parent = "example.com".to_string();
        let actor_id = Uuid::parse_str("55555555-5555-5555-5555-555555555555").unwrap();
        let port_name = "api";
        let protocol = GameGuardProtocol::Https;

        // Build hostname and path using build_actor_hostname_and_path
        let (hostname, path) = build_actor_hostname_and_path(
            actor_id,
            port_name,
            protocol,
            EndpointType::Path,
            &GuardPublicHostname::DnsParent(dns_parent.clone())
        ).unwrap();

        // Build regex using build_actor_hostname_and_path_regex
        let (hostname_regex, path_regex_opt) = build_actor_hostname_and_path_regex(
            EndpointType::Path,
            &GuardPublicHostname::DnsParent(dns_parent)
        ).unwrap();

        // Assert that path exists
        assert!(path.is_some());
        assert!(path_regex_opt.is_some());
        let path_regex = path_regex_opt.unwrap();

        // Match hostname against hostname regex
        assert!(hostname_regex.is_match(&hostname));

        // Match path against path regex
        let path_str = path.unwrap();
        let captures = path_regex.captures(&path_str).unwrap();
        assert_eq!(captures.name("actor_id").unwrap().as_str(), "55555555-5555-5555-5555-555555555555");
        assert_eq!(captures.name("port_name").unwrap().as_str(), port_name);
    }

    #[test]
    fn test_static_hostname_path_from_build_actor_hostname_with_regex() {
        // Test static hostname with path routing
        let static_hostname = "static.example.org".to_string();
        let actor_id = Uuid::parse_str("66666666-6666-6666-6666-666666666666").unwrap();
        let port_name = "game";
        let protocol = GameGuardProtocol::Http;

        // Build hostname and path using build_actor_hostname_and_path
        let (hostname, path) = build_actor_hostname_and_path(
            actor_id,
            port_name,
            protocol,
            EndpointType::Path,
            &GuardPublicHostname::Static(static_hostname.clone())
        ).unwrap();

        // Build regex using build_actor_hostname_and_path_regex
        let (hostname_regex, path_regex_opt) = build_actor_hostname_and_path_regex(
            EndpointType::Path,
            &GuardPublicHostname::Static(static_hostname)
        ).unwrap();

        // Assert that path exists
        assert!(path.is_some());
        assert!(path_regex_opt.is_some());
        let path_regex = path_regex_opt.unwrap();

        // Match hostname against hostname regex
        assert!(hostname_regex.is_match(&hostname));

        // Match path against path regex
        let path_str = path.unwrap();
        let captures = path_regex.captures(&path_str).unwrap();
        assert_eq!(captures.name("actor_id").unwrap().as_str(), "66666666-6666-6666-6666-666666666666");
        assert_eq!(captures.name("port_name").unwrap().as_str(), port_name);
    }
}
