# Prerequisite:
0. **Update**
    ```bash
    sudo apt update
    sudo apt upgrade -y
    ```
1. **Nginx**
    Install `nginx`
    ```bash
    sudo apt install nginx -y
    ```

    On the VPS, make sure to create the folders:
    | dev                  | live                 |
    |----------------------|----------------------|
    | `/var/dev/portfolio` | `/var/www/portfolio` |
    | `/var/dev/blog`      | `/var/www/blog`      |

    Generate a github-actions key-pair
    ```bash
    ssh-keygen -t ed25519 -C "github-actions" -f github_actions_key -N ""
    ```
3. **Certbot**
    Install `certbot` and its plugin for nginx
    ```bash
    sudo apt install certbot python3-certbot-nginx -y
    ```

    Sometime `apache` or something else might use the port 80, you can check the port and get rid of them if exist
    ```bash
    sudo lsof -i :80
    ```

    Sign the sub domains, be sure they are correctly assigned on your dns manager.
    ```bash
    sudo certbot --nginx -d portfolio.huuthang.site -d blog.huuthang.site
    ```

    Create the configs for the portfolio and blog
    ```bash
    sudo nano /etc/nginx/sites-available/portfolio
    sudo nano /etc/nginx/sites-available/blog
    ```

    Link them to the sites-enabled to make it enabled
    ```bash
    sudo ln -s /etc/nginx/sites-available/portfolio /etc/nginx/sites-enabled/
    sudo ln -s /etc/nginx/sites-available/blog /etc/nginx/sites-enabled/
    ```
    *(the reference files can be found here: [portfolio](./nginx/portfolio) and [blog](./nginx/blog))*

    Test and reload
    ```bash
    sudo nginx -t
    sudo systemctl reload nginx
    ```

4. **Portfolio: React**
    Install `npm` and `bun`
    ```bash
    sudo apt install npm -y
    npm install -g bun
    ```

5. **Blog: Blazor server app** 
    Install `dotnet`
    ```bash
    sudo add-apt-repository ppa:dotnet/backports
    sudo apt-get install dotnet-sdk-9.0 -y
    sudo apt-get install aspnetcore-runtime-9.0 -y
    ```

    The Blog is different from the Portfolio, we need a service to run in the background on a certain port and tell `nginx` to reverse proxy to that service.
    
    The reference service can be found [here](./services/blog.service). Create such file at `/etc/systemd/system/blog.service` then execute
    ```bash
    sudo systemctl daemon-reload
    sudo systemctl enable --now blog.service
    ```
    To check the service:
    ```bash
    sudo systemctl status blog.service
    ```
6. **(new) Blog: Svelte+Rust server app**
    Fuck Blazor, please remove the service
    ```bash
    sudo systemctl stop blog.service
    sudo systemctl disable blog.service
    sudo rm /etc/systemd/system/blog.service
    sudo systemctl daemon-reload
    sudo systemctl reset-failed
    ```
    Remove the .NET runtime
    ```bash
    sudo apt remove --purge dotnet-sdk-9.0 aspnetcore-runtime-9.0 -y
    sudo apt autoremove -y
    ```
    
    ### *ADD A RATE LIMITER*
    inside `/etc/nginx/nginx.conf`
    ```bash
    http {
        # Add this line here:
        limit_req_zone $binary_remote_addr zone=blog:10m rate=5r/s;
    
        # ... leave everything else as it is ...
    }
    ```
    Dont forget to update your blog config
