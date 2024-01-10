#

```bash
sudo apt-get update
sudo apt-get upgrade --yes
sudo apt-get install --yes libfluidsynth2 fluid-soundfont-gm
sudo alsamixer
sudo systemctl disable avahi-daemon.service
sudo systemctl disable dhcpcd.service
sudo systemctl disable e2scrub_reap.service
sudo systemctl disable keyboard-setup.service
sudo systemctl disable ModemManager.service
sudo systemctl disable systemd-timesyncd.service
sudo systemctl disable triggerhappy.service
sudo systemctl disable wpa_supplicant.service
sudo vi /etc/network/interfaces.d/home.conf
```

```/etc/network/interfaces.d/home.conf
auto eth0

iface eth0 inet static
address 192.168.x.y
network 192.168.x.z
netmask 255.255.255.0
broadcast 192.168.x.255
gateway 192.168.x.z
dns-nameservers 192.168.x.z
```

```powershell
deploy.ps1
```

```bash
sudo raspi-config nonint enable_overlayfs
```
