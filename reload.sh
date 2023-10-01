echo "Stopping service"
sudo systemctl stop website.service

echo "Pulling from Github"
git pull

echo "Building"
cargo update
cargo build --release

echo "Starting service"
sudo systemctl start website.service
echo "Enabling on reboot"
sudo systemctl enable website.service