id: health
name: Health Check
summary: Submit a Docker image that responds to health checks
points: 10
prerequisites: [intro]
is_hidden: false

---

Before the Council can trust your exchange with real energy trades, they need proof that your service can even stay alive.
The old exchange crashed seventeen times in its final week - each outage causing minor blackouts across three star systems.
The Council's automated monitoring drones will periodically ping your service to verify it's still operational.

Your first mission is to submit a prototype which responds to health checks to prove that your service works in the Council's conformance testing environment.

## Docker image submission

Each team is given their own repository, where they can push their Docker images. Once you build your image, login to the registry and push it.
The push will automatically trigger a series of events that will eventually lead to updating the UI with the option to submit the image for testing.

Be sure to build the dockerfile for platform `linux/amd64`.

**Resource limits:** Your container will run with **2 CPU cores**, **2 GB RAM**, and **5 GB storage**.

```bash
# Login to the registry
# https://docs.docker.com/
docker login registry.k8s.energyhack.cz -u user.name

# Build your galactic exchange image
# If your team repository name is 'energy-hack-submission' your tag needs to be
# registry.k8s.energyhack.cz/energy-hack-submission/galactic-energy-exchange:tag
# Tag can be anything you want, for example :v1.0.0, :latest or :abc123f.
docker build -t registry.k8s.energyhack.cz/team-repo/galactic-energy-exchange:latest .

# Push the image to the registry
docker push registry.k8s.energyhack.cz/team-repo/galactic-energy-exchange:latest
```

You can review and manage your images in the Harbor UI at https://registry.k8s.energyhack.cz/.

## Endpoint Specification

To maintain compatibility with the aging systems still powering the old exchange, you must rely on an ancient protocol long past its prime: HTTP. According to a dusty decree from the historical Protocol Committee, this relic may only operate on TCP port 8080. This is a rule nobody fully understands anymore, yet all infrastructure stubbornly obeys.

[HTTP Specification](https://httpwg.org/specs/)

**GET /health**

**Response:**
- Success: `200 OK` (empty response body or any response body)

### Status Codes

| HTTP Status | Scenario                    |
|-------------|-----------------------------|
| 200 OK      | Service is healthy          |
