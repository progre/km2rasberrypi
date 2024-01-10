param (
  [string]${projectName},
  [string]$remoteHost
)

ssh ${remoteHost} "mkdir -p /tmp/${projectName}/"
scp target/aarch64-unknown-linux-gnu/debug/${projectName}* ${remoteHost}:/tmp/${projectName}/
ssh -t ${remoteHost} "chmod +x /tmp/${projectName}/${projectName} && sudo /tmp/${projectName}/${projectName}"
