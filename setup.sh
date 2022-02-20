echo "Copying service file to /etc/systemd/system/"
cp /root/website/website.service /etc/systemd/system/website.service
echo "Reloading service files"
sudo systemctl start your-service.service
echo "Enabling on reboot"
sudo systemctl enable website.service