source .env

#ssh -t ${SSH_USER_ADDR} "rm -rf ~/hardware-monitoring"
ssh -t ${SSH_USER_ADDR} "mkdir -p ~/hardware-monitoring"

scp -r ../backend/src ${SSH_USER_ADDR}:~/hardware-monitoring/
scp ../backend/Cargo.lock ${SSH_USER_ADDR}:~/hardware-monitoring/
scp ../backend/Cargo.toml ${SSH_USER_ADDR}:~/hardware-monitoring/

ssh -t ${SSH_USER_ADDR} "cd hardware-monitoring && ~/.cargo/bin/cargo run < <(cat; kill -INT 0); echo done"

#ssh ${SSH_USER_ADDR} "sh -c cd monitoring && cargo run"
