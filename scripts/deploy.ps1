param (
  [string]${projectName},
  [string]$remoteHost
)

ssh "${remoteHost}" "mkdir -p /tmp/${projectName}/"
scp "target/aarch64-unknown-linux-gnu/release/${projectName}" "${remoteHost}:/tmp/${projectName}/"
scp "${projectName}.service" "${remoteHost}:/tmp/${projectName}/"
ssh "${remoteHost}" @"
chmod +x /tmp/${projectName}/${projectName} &&
sudo mv /tmp/${projectName}/${projectName} /usr/local/bin/ &&
sudo mv /tmp/${projectName}/${projectName}.service /etc/systemd/system/ &&
sudo systemctl daemon-reload &&
sudo systemctl start ${projectName} &&
sudo systemctl enable ${projectName}
"@
