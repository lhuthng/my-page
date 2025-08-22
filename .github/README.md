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

4. **NPM + Bun**
    Install `npm` and `bun`
    ```bash
    sudo apt install npm -y
    npm install -g bun
    ```

   