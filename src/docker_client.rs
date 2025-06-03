use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

use crate::types::Container;

pub struct DockerClient {
    socket_path: String,
}

impl DockerClient {
    pub fn new(socket_path: String) -> Self {
        Self { socket_path }
    }

    async fn api_call(&self, endpoint: &str) -> Result<String, Box<dyn std::error::Error>> {
        let stream = UnixStream::connect(&self.socket_path)?;
        self.send_request(stream, endpoint).await
    }

    async fn send_request(
        &self,
        mut stream: UnixStream,
        endpoint: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let request = format!(
            "GET {} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
            endpoint
        );

        stream.write_all(request.as_bytes())?;
        self.read_response(stream)
    }

    fn read_response(&self, mut stream: UnixStream) -> Result<String, Box<dyn std::error::Error>> {
        let mut response = String::new();
        stream.read_to_string(&mut response)?;

        // Clean up HTTP chunked encoding and extract JSON body
        if let Some(json_start) = response.find("\r\n\r\n") {
            let body = &response[json_start + 4..];
            // Handle chunked encoding - remove chunk size markers
            Ok(self.clean_chunked_response(body))
        } else {
            Ok(response)
        }
    }

    fn clean_chunked_response(&self, body: &str) -> String {
        // Remove HTTP chunked encoding artifacts
        let mut cleaned = body.to_string();

        // Remove chunk size at the beginning (like "f053\r\n")
        if let Some(first_newline) = cleaned.find("\r\n") {
            if cleaned[..first_newline]
                .chars()
                .all(|c| c.is_ascii_hexdigit())
            {
                cleaned = cleaned[first_newline + 2..].to_string();
            }
        }

        // Remove trailing chunk markers (like "\r\n0\r\n\r\n")
        if cleaned.ends_with("\r\n0\r\n\r\n") {
            cleaned.truncate(cleaned.len() - 7);
        } else if cleaned.ends_with("\n\r\n0\r\n\r\n") {
            cleaned.truncate(cleaned.len() - 8);
        }

        cleaned
    }

    pub async fn list_containers(
        &self,
        all: bool,
    ) -> Result<Vec<Container>, Box<dyn std::error::Error>> {
        let endpoint = if all {
            "/containers/json?all=true"
        } else {
            "/containers/json"
        };
        let json_response = self.api_call(endpoint).await?;
        let containers: Vec<Container> = serde_json::from_str(&json_response)?;
        Ok(containers)
    }

    pub async fn get_running_containers_by_image_substring(
        &self,
        image_substring: &str,
    ) -> Result<Vec<Container>, Box<dyn std::error::Error>> {
        let containers = self.list_containers(true).await?;
        Ok(containers
            .into_iter()
            .filter(|container| {
                container.state == "running" && container.image.contains(image_substring)
            })
            .collect())
    }

    pub async fn remove_container(
        &self,
        container_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let endpoint = &format!("/containers/{}?force=true", container_id);
        let stream = UnixStream::connect(&self.socket_path)?;
        self.send_delete_request(stream, endpoint).await?;
        Ok(())
    }

    async fn send_delete_request(
        &self,
        mut stream: UnixStream,
        endpoint: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let request = format!(
            "DELETE {} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
            endpoint
        );

        stream.write_all(request.as_bytes())?;
        self.read_response(stream)
    }

    pub async fn stop_container(
        &self,
        container_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let endpoint = &format!("/containers/{}/stop", container_id);
        let stream = UnixStream::connect(&self.socket_path)?;
        self.send_post_request(stream, endpoint, "").await?;
        Ok(())
    }

    pub async fn start_container(
        &self,
        container_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let endpoint = &format!("/containers/{}/start", container_id);
        let stream = UnixStream::connect(&self.socket_path)?;
        self.send_post_request(stream, endpoint, "").await?;
        Ok(())
    }

    async fn send_post_request(
        &self,
        mut stream: UnixStream,
        endpoint: &str,
        body: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let request = format!(
            "POST {} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            endpoint,
            body.len(),
            body
        );

        stream.write_all(request.as_bytes())?;
        self.read_response(stream)
    }

    pub async fn get_running_containers_by_name(
        &self,
        name: &str,
    ) -> Result<Vec<Container>, Box<dyn std::error::Error>> {
        let containers = self.list_containers(true).await?;
        Ok(containers
            .into_iter()
            .filter(|container| {
                container.state == "running" && container.names.iter().any(|n| n.contains(name))
            })
            .collect())
    }
}
