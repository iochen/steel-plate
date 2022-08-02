# Steel Plate
Project Steel Plate for Arctic Fox (or 雪狐桑)

## Volunteer
You can help improve the UI design by pulling request.   
Additionally, authorized media materials will also be highly appreciated.   

## Tested Browsers
1. Safari (has a known bug)
2. Google Chrome

## Installation
### AWS Lambda
#### Build
##### Mac OS
(Tested on Apple Silicon ONLY)
1. Install Rust develop environment 
2. Add target `x86_64-unknown-linux-musl` with command below:
```shell
$ rustup target add x86_64-unknown-linux-musl
```
3. Install ross-compilation toolchain
```shell
$ brew install filosottile/musl-cross/musl-cross
```
4. Navigate into project root, then run
```shell
$ mkdir .cargo
$ echo '[target.x86_64-unknown-linux-musl]
linker = "x86_64-linux-musl-gcc"' > .cargo/config
```
5. Add command alias for `x86_64-linux-musl-gcc`
```shell
$ ln -s `where x86_64-linux-musl-gcc` /usr/local/bin/musl-gcc
```
6. Cross compile
```shell
$ cargo build --release --target x86_64-unknown-linux-musl
```
7. Pack as zip file
```shell
$ zip -j rust.zip ./target/x86_64-unknown-linux-musl/release/bootstrap
```
##### Linux amd64
!!! TO BE TESTED !!!
1. Install Rust develop environment 
2. Navigate into project root
3. Compile
```shell
$ cargo build --release
```
4. Pack as zip file
```shell
$ zip -j rust.zip ./target/release/bootstrap
```
#### Lambda Prepare
1. Click `Create function` on Lambda Functions page
2. Choose `Author from scratch`
3. Enter your function name
4. Set `Runtime` to `Provide your own bootstrap on Amazon Linux 2`
5. Set `Architecture` to `x86_64`
6. Click `Create function` to finish create
7. Go into the function you created, click `Upload from` under card `Code source`
8. Choose `.zip file`
9. Upload `rust.zip` packed in the previous step, and click `Save`
10. Wait until it finishes
#### DynamoDB Prepare
1. Click `Create table` on DynamoDB Tables page
2. Enter your `Table name`
3. Set `Partition key` to `key` with type `String`
4. Set `Table settings` according to your needs
5. Click `Create table` to finish create
6. Click `Explore items` on sidebar
7. Choose the `Table name` you set before
8. Click `Create item`
9. Under `Add new attribute`, choose `Number`
10. Set **Value** to `total` where **Attribute** name shows `key`
11. Change `NewValue` to `value` and set its **Value** to `0`
12. Click `Create item` to finish
#### Edit roles
1. Go to **IAM** and open **Roles**
2. Choose the role bound to the lambda function you created before
3. Click `Attach policies` under `Add permissions`
4. Either create your own policy (safer, more complex), or use `AmazonDynamoDBFullAccess` provides by AWS (more dangerous, but easier)
5. Click `Attach policies` to finish role edition
Example custom policy:
```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "0",
      "Effect": "Allow",
      "Action": [
        "dynamodb:PutItem",
        "dynamodb:GetItem",
        "dynamodb:UpdateItem"
      ],
      "Resource": "arn:aws:dynamodb:xxxxxx:xxxxxxx:table/xxxxxx"
    },
    {
      "Sid": "1",
      "Effect": "Allow",
      "Action": "dynamodb:ListTables",
      "Resource": "*"
    }
  ]
}
```
#### Api Gateway Setup
1. Create a `HTTP API`
2. Set `Integrations` to your lambda function
3. Under `Configure routes`, set **Method** to `ANY`, **Resource path** to `/{proxy+}` and **Integration target** to your lambda function 
4. Set others as default or edit them to your preference
5. Open the URL showed in `Invoke URL`, and you can finally enjoy the 100% pure steel plate of Arctic Fox

### Standalone
!!! Standalone will store total clicks in the server memory and ONLY in the memory !!!   
However, by setting env `STEEL_PLATE_COUNT_BASE`, you can set the base clicks count number
   
1. Install Rust development environment
2. Navigate into project root
3. Compile with command below:
```shell
$ cargo build --release
```
4. Run:
```shell
$ ./target/release/steel-plate
```
5. Enjoy the high-purity steel plate of Arctic Fox at `http://127.0.0.1:8082/`

## Known Bugs
1. Unable to play the audio after quick clicks on Safari

## External Links
- Arctic Fox (or 雪狐桑): [Bilibili UID:477792](https://space.bilibili.com/477792)

## LICENSE
MIT LICENSE   
2022 Richard Chen