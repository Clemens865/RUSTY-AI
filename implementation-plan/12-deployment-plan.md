# Deployment Plan & Scaling Strategy

## Overview

Comprehensive deployment strategy for the Personal AI Assistant, covering containerization, orchestration, scaling, and infrastructure management across development, staging, and production environments.

## Deployment Architecture

### 1. Containerization Strategy

#### Multi-stage Dockerfile

```dockerfile
# Build stage
FROM rust:1.75-slim as builder

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libasound2-dev \
    portaudio19-dev \
    cmake \
    && rm -rf /var/lib/apt/lists/*

# Copy dependency files
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Build dependencies (cached layer)
RUN cargo build --release --bin personal-ai-assistant

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libasound2 \
    portaudio19-dev \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -u 1000 assistant

# Copy application binary
COPY --from=builder /app/target/release/personal-ai-assistant /usr/local/bin/

# Copy models and config
COPY --from=builder /app/models /app/models
COPY --from=builder /app/config /app/config

# Set ownership
RUN chown -R assistant:assistant /app

USER assistant
WORKDIR /app

EXPOSE 8080

CMD ["personal-ai-assistant"]
```

#### Docker Compose for Development

```yaml
# docker-compose.yml
version: '3.8'

services:
  assistant:
    build:
      context: .
      dockerfile: Dockerfile
    ports:
      - "8080:8080"
    environment:
      - RUST_LOG=debug
      - DATABASE_URL=postgresql://postgres:password@postgres:5432/assistant_dev
      - QDRANT_URL=http://qdrant:6333
      - REDIS_URL=redis://redis:6379
    volumes:
      - ./data:/app/data
      - ./logs:/app/logs
    depends_on:
      - postgres
      - qdrant
      - redis
    networks:
      - assistant-network

  postgres:
    image: postgres:15-alpine
    environment:
      POSTGRES_DB: assistant_dev
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: password
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./scripts/init-db.sql:/docker-entrypoint-initdb.d/init.sql
    ports:
      - "5432:5432"
    networks:
      - assistant-network

  qdrant:
    image: qdrant/qdrant:v1.7.4
    ports:
      - "6333:6333"
      - "6334:6334"
    volumes:
      - qdrant_data:/qdrant/storage
    networks:
      - assistant-network

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data
    networks:
      - assistant-network

  frontend:
    build:
      context: ./frontend/vox-chic-studio
      dockerfile: Dockerfile
    ports:
      - "3000:3000"
    environment:
      - NEXT_PUBLIC_API_BASE_URL=http://localhost:8080/api/v1
      - NEXT_PUBLIC_WS_URL=ws://localhost:8080/ws
    depends_on:
      - assistant
    networks:
      - assistant-network

volumes:
  postgres_data:
  qdrant_data:
  redis_data:

networks:
  assistant-network:
    driver: bridge
```

### 2. Kubernetes Deployment

#### Namespace Configuration

```yaml
# k8s/namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: personal-ai-assistant
  labels:
    name: personal-ai-assistant
    environment: production
```

#### ConfigMap and Secrets

```yaml
# k8s/configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: assistant-config
  namespace: personal-ai-assistant
data:
  RUST_LOG: "info"
  APP_HOST: "0.0.0.0"
  APP_PORT: "8080"
  QDRANT_URL: "http://qdrant-service:6333"
  REDIS_URL: "redis://redis-service:6379"
  
---
apiVersion: v1
kind: Secret
metadata:
  name: assistant-secrets
  namespace: personal-ai-assistant
type: Opaque
data:
  DATABASE_URL: # Base64 encoded database URL
  ELEVENLABS_API_KEY: # Base64 encoded API key
  JWT_SECRET: # Base64 encoded JWT secret
  ENCRYPTION_KEY: # Base64 encoded encryption key
```

#### Application Deployment

```yaml
# k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: assistant-api
  namespace: personal-ai-assistant
  labels:
    app: assistant-api
spec:
  replicas: 3
  selector:
    matchLabels:
      app: assistant-api
  template:
    metadata:
      labels:
        app: assistant-api
    spec:
      containers:
      - name: assistant-api
        image: personal-ai-assistant:latest
        imagePullPolicy: Always
        ports:
        - containerPort: 8080
        envFrom:
        - configMapRef:
            name: assistant-config
        - secretRef:
            name: assistant-secrets
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "2000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
        volumeMounts:
        - name: models
          mountPath: /app/models
          readOnly: true
        - name: logs
          mountPath: /app/logs
      volumes:
      - name: models
        persistentVolumeClaim:
          claimName: models-pvc
      - name: logs
        persistentVolumeClaim:
          claimName: logs-pvc
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        fsGroup: 1000
```

#### Service and Ingress

```yaml
# k8s/service.yaml
apiVersion: v1
kind: Service
metadata:
  name: assistant-service
  namespace: personal-ai-assistant
spec:
  selector:
    app: assistant-api
  ports:
  - port: 80
    targetPort: 8080
    protocol: TCP
  type: ClusterIP

---
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: assistant-ingress
  namespace: personal-ai-assistant
  annotations:
    kubernetes.io/ingress.class: nginx
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/rate-limit: "100"
    nginx.ingress.kubernetes.io/rate-limit-window: "1m"
spec:
  tls:
  - hosts:
    - api.personal-assistant.com
    secretName: assistant-tls
  rules:
  - host: api.personal-assistant.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: assistant-service
            port:
              number: 80
```

#### Horizontal Pod Autoscaler

```yaml
# k8s/hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: assistant-hpa
  namespace: personal-ai-assistant
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: assistant-api
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
  behavior:
    scaleUp:
      stabilizationWindowSeconds: 30
      policies:
      - type: Percent
        value: 50
        periodSeconds: 60
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
      - type: Percent
        value: 25
        periodSeconds: 60
```

### 3. Database Deployment

#### PostgreSQL with High Availability

```yaml
# k8s/postgres-cluster.yaml
apiVersion: postgresql.cnpg.io/v1
kind: Cluster
metadata:
  name: postgres-cluster
  namespace: personal-ai-assistant
spec:
  instances: 3
  primaryUpdateStrategy: unsupervised
  
  postgresql:
    parameters:
      max_connections: "200"
      shared_buffers: "256MB"
      effective_cache_size: "1GB"
      maintenance_work_mem: "64MB"
      checkpoint_completion_target: "0.9"
      wal_buffers: "16MB"
      default_statistics_target: "100"
      random_page_cost: "1.1"
      effective_io_concurrency: "200"
  
  bootstrap:
    initdb:
      database: assistant_prod
      owner: assistant_user
      secret:
        name: postgres-credentials
  
  storage:
    size: 100Gi
    storageClass: fast-ssd
  
  monitoring:
    enabled: true
    prometheusRule:
      enabled: true
  
  backup:
    retentionPolicy: "30d"
    barmanObjectStore:
      destinationPath: "s3://assistant-backups/postgres"
      s3Credentials:
        accessKeyId:
          name: backup-credentials
          key: ACCESS_KEY_ID
        secretAccessKey:
          name: backup-credentials
          key: SECRET_ACCESS_KEY
```

#### Qdrant Vector Database

```yaml
# k8s/qdrant.yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: qdrant
  namespace: personal-ai-assistant
spec:
  serviceName: qdrant-service
  replicas: 3
  selector:
    matchLabels:
      app: qdrant
  template:
    metadata:
      labels:
        app: qdrant
    spec:
      containers:
      - name: qdrant
        image: qdrant/qdrant:v1.7.4
        ports:
        - containerPort: 6333
        - containerPort: 6334
        env:
        - name: QDRANT__CLUSTER__ENABLED
          value: "true"
        - name: QDRANT__CLUSTER__P2P__PORT
          value: "6335"
        volumeMounts:
        - name: qdrant-storage
          mountPath: /qdrant/storage
        resources:
          requests:
            memory: "1Gi"
            cpu: "500m"
          limits:
            memory: "4Gi"
            cpu: "2000m"
  volumeClaimTemplates:
  - metadata:
      name: qdrant-storage
    spec:
      accessModes: ["ReadWriteOnce"]
      storageClassName: fast-ssd
      resources:
        requests:
          storage: 50Gi
```

## Infrastructure as Code

### 1. Terraform Configuration

```hcl
# terraform/main.tf
terraform {
  required_version = ">= 1.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "~> 2.0"
    }
  }
  
  backend "s3" {
    bucket = "personal-assistant-terraform-state"
    key    = "production/terraform.tfstate"
    region = "us-west-2"
  }
}

provider "aws" {
  region = var.aws_region
}

# EKS Cluster
module "eks" {
  source = "terraform-aws-modules/eks/aws"
  
  cluster_name    = var.cluster_name
  cluster_version = "1.28"
  
  vpc_id     = module.vpc.vpc_id
  subnet_ids = module.vpc.private_subnets
  
  eks_managed_node_groups = {
    general = {
      desired_size = 3
      min_size     = 2
      max_size     = 10
      
      instance_types = ["t3.large"]
      capacity_type  = "ON_DEMAND"
      
      k8s_labels = {
        Environment = var.environment
        Application = "personal-ai-assistant"
      }
    }
    
    compute_intensive = {
      desired_size = 2
      min_size     = 1
      max_size     = 5
      
      instance_types = ["c5.2xlarge"]
      capacity_type  = "SPOT"
      
      k8s_labels = {
        Environment = var.environment
        Application = "personal-ai-assistant"
        WorkloadType = "compute-intensive"
      }
      
      taints = {
        dedicated = {
          key    = "compute-intensive"
          value  = "true"
          effect = "NO_SCHEDULE"
        }
      }
    }
  }
}

# RDS for PostgreSQL
module "rds" {
  source = "terraform-aws-modules/rds/aws"
  
  identifier = "${var.cluster_name}-postgres"
  
  engine            = "postgres"
  engine_version    = "15.4"
  instance_class    = "db.t3.large"
  allocated_storage = 100
  storage_encrypted = true
  
  db_name  = "assistant_prod"
  username = var.db_username
  password = var.db_password
  
  vpc_security_group_ids = [aws_security_group.rds.id]
  db_subnet_group_name   = module.vpc.database_subnet_group
  
  backup_retention_period = 30
  backup_window          = "03:00-04:00"
  maintenance_window     = "sun:04:00-sun:05:00"
  
  monitoring_interval = 60
  monitoring_role_arn = aws_iam_role.rds_monitoring.arn
  
  performance_insights_enabled = true
  
  tags = {
    Environment = var.environment
    Application = "personal-ai-assistant"
  }
}
```

### 2. Helm Charts

```yaml
# helm/personal-ai-assistant/Chart.yaml
apiVersion: v2
name: personal-ai-assistant
description: Personal AI Assistant Helm Chart
type: application
version: 0.1.0
appVersion: "1.0.0"

dependencies:
- name: postgresql
  version: 12.x.x
  repository: https://charts.bitnami.com/bitnami
  condition: postgresql.enabled
- name: redis
  version: 17.x.x
  repository: https://charts.bitnami.com/bitnami
  condition: redis.enabled
```

```yaml
# helm/personal-ai-assistant/values.yaml
image:
  repository: personal-ai-assistant
  tag: latest
  pullPolicy: IfNotPresent

replicaCount: 3

service:
  type: ClusterIP
  port: 80
  targetPort: 8080

ingress:
  enabled: true
  className: nginx
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/rate-limit: "100"
  hosts:
    - host: api.personal-assistant.com
      paths:
        - path: /
          pathType: Prefix
  tls:
    - secretName: assistant-tls
      hosts:
        - api.personal-assistant.com

autoscaling:
  enabled: true
  minReplicas: 3
  maxReplicas: 10
  targetCPUUtilizationPercentage: 70
  targetMemoryUtilizationPercentage: 80

resources:
  requests:
    memory: "512Mi"
    cpu: "500m"
  limits:
    memory: "2Gi"
    cpu: "2000m"

postgresql:
  enabled: true
  auth:
    database: assistant_prod
    username: assistant_user
  primary:
    persistence:
      enabled: true
      size: 100Gi
      storageClass: fast-ssd

redis:
  enabled: true
  auth:
    enabled: true
  master:
    persistence:
      enabled: true
      size: 10Gi
      storageClass: fast-ssd
```

## Scaling Strategies

### 1. Horizontal Scaling

```rust
// Application-level scaling considerations
pub struct ScalingManager {
    metrics_collector: Arc<MetricsCollector>,
    load_balancer: Arc<LoadBalancer>,
    resource_monitor: Arc<ResourceMonitor>,
}

impl ScalingManager {
    pub async fn monitor_and_scale(&self) -> Result<(), ScalingError> {
        let metrics = self.metrics_collector.get_current_metrics().await?;
        
        if metrics.cpu_utilization > 0.8 || metrics.memory_utilization > 0.85 {
            self.trigger_scale_up().await?;
        } else if metrics.cpu_utilization < 0.3 && metrics.memory_utilization < 0.5 {
            self.trigger_scale_down().await?;
        }
        
        Ok(())
    }
    
    async fn trigger_scale_up(&self) -> Result<(), ScalingError> {
        // Trigger Kubernetes HPA or custom scaling logic
        self.request_additional_replicas(2).await?;
        
        // Update load balancer configuration
        self.load_balancer.redistribute_load().await?;
        
        Ok(())
    }
}
```

### 2. Vertical Scaling

```yaml
# k8s/vpa.yaml
apiVersion: autoscaling.k8s.io/v1
kind: VerticalPodAutoscaler
metadata:
  name: assistant-vpa
  namespace: personal-ai-assistant
spec:
  targetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: assistant-api
  updatePolicy:
    updateMode: "Auto"
  resourcePolicy:
    containerPolicies:
    - containerName: assistant-api
      maxAllowed:
        cpu: 4
        memory: 8Gi
      minAllowed:
        cpu: 100m
        memory: 128Mi
```

## Blue-Green Deployment

```bash
#!/bin/bash
# scripts/blue-green-deploy.sh

set -e

NAMESPACE="personal-ai-assistant"
NEW_VERSION="$1"
CURRENT_COLOR=$(kubectl get service assistant-service -n $NAMESPACE -o jsonpath='{.spec.selector.color}')

if [ "$CURRENT_COLOR" = "blue" ]; then
    NEW_COLOR="green"
else
    NEW_COLOR="blue"
fi

echo "Current deployment: $CURRENT_COLOR"
echo "Deploying to: $NEW_COLOR"

# Deploy new version
kubectl set image deployment/assistant-$NEW_COLOR assistant-api=personal-ai-assistant:$NEW_VERSION -n $NAMESPACE

# Wait for rollout
kubectl rollout status deployment/assistant-$NEW_COLOR -n $NAMESPACE

# Health check
echo "Performing health checks..."
for i in {1..30}; do
    if kubectl run health-check --rm -i --restart=Never --image=curlimages/curl -- \
        curl -f http://assistant-$NEW_COLOR-service:80/health; then
        echo "Health check passed"
        break
    fi
    echo "Health check attempt $i failed, retrying..."
    sleep 10
done

# Switch traffic
echo "Switching traffic to $NEW_COLOR"
kubectl patch service assistant-service -n $NAMESPACE -p '{"spec":{"selector":{"color":"'$NEW_COLOR'"}}}'

# Wait and verify
sleep 30

# Scale down old deployment
echo "Scaling down $CURRENT_COLOR deployment"
kubectl scale deployment assistant-$CURRENT_COLOR --replicas=0 -n $NAMESPACE

echo "Deployment completed successfully"
```

This deployment plan ensures reliable, scalable, and maintainable infrastructure for the Personal AI Assistant across all environments.