echo "Copying service file to /etc/systemd/system/"
cp /root/website/website.service /etc/systemd/system/website.service
echo "Reloading service files"
sudo systemctl daemon-reload
echo "Starting service"
sudo systemctl start website.service
echo "Enabling on reboot"
sudo systemctl enable website.service