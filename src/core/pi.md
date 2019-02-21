# Installing on the Raspberry Pi

## Installing Raspbian

First, flash a modern version of [Raspbian][] onto an SD card.
The "lite" version should suffice.
I use [Etcher][] to flash SD cards, but you can also [use `dd`][dd] from the command line.

## Editing the config

After flashing, open the drive (which will be mounted as `/boot` on the pi) and copy over an `authorized_keys` file with ssh public keys in it.
Also, open `config.txt` and edit/uncomment it so that the following lines are there:
```
dtparam=i2c_arm=on
dtparam=spi=on
dtoverlay=pwm
```

## Initial pi setup

Plug in a keyboard and monitor to the pi and boot it.
Using the Pi Configuration utility:
- set the hostname of the Pi to something.
- enable the camera, SSH, and VNC interfaces.

Now let's make a regular user instead of the default `pi` user:
```shell
# make a regular user, in this case named mwillsey
# also give the user sudo permissions and the ability to use gpio pins and the camera
sudo adduser mwillsey
sudo usermod mwillsey -aG sudo,spi,i2c,gpio,video

# become the regular user
su mwillsey

# make a .ssh directory, give it the right permissions
mkdir ~/.ssh
chmod 700 .ssh

# copy the keys file into the .ssh directory, and give it the right perms
# after this, you'll be able to ssh into the pi
sudo cp /boot/authorized_keys ~/.ssh/authorized_keys
sudo chown mwillsey:mwillsey ~/.ssh/authorized_keys
sudo chmod 600 ~/.ssh/authorized_keys
```

Also, record the hostname by making a file on the SD card:
`sudo touch /boot/HOST-<hostname>`.
This is just so you can see which Pi your talking about if you take the SD card out.

## Networking

Now you've got to join the internet somehow. 
For some reason, you can't join eduroam from a Pi.
If you're joining the UW wifi, I'd just 
[register the MAC address][mac].
You can find the MAC address by running `ip a` and looking for the line under `wlan0`.

Now join your [ZeroTier][] network.

```shell
# install zerotier
curl -s https://install.zerotier.com/ | sudo bash
sudo zerotier-cli join <network>
sudo zerotier-cli status
```

Secure your SSH by setting `PasswordAuthentication no` in `/etc/ssh/sshd_config`. Make sure you can ssh in before you do this.

Secure the VNC interface by opening it, going to the menu, options, and then connections. Set the rules to accept only from your ZeroTier subnet, and reject by default.

Finally, enable PWM for regular users (without `sudo`) according to these [instructions][pwm].


[raspbian]: https://www.raspberrypi.org/downloads/raspbian/
[etcher]: https://www.balena.io/etcher/
[dd]: https://www.raspberrypi.org/documentation/installation/installing-images/linux.md
[mac]: https://cppm-uwtc-01.infra.washington.edu/guest/mac_create.php
[zerotier]: https://www.zerotier.com/
[pwm]: https://docs.golemparts.com/rppal/0.10.0/rppal/pwm/index.html
