### systemd

Systemd unit to keep the docker containers alive in CoreOS.

 - Copy `boot.service` to `/etc/systemd/system`
 - Run `sudo systemctl enable /etc/systemd/system/boot.service` (which will create a symlink)
 - Then, run `sudo systemctl start boot.service` to try running the unit, and `journalctl -f -u boot.service` to check the log.
