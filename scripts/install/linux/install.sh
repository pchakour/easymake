mkdir $HOME/.emake
cd $HOME/.emake
wget https://github.com/pchakour/easymake/releases/download/v0.0.1/emake-linux.tar.gz
tar -xvf emake-linux.tar.gz
rm emake-linux.tar.gz
echo '. "$HOME/.emake/env"' >> $HOME/.profile