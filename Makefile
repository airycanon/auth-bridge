dev:
	kustomize build config/default | kubectl apply -f -