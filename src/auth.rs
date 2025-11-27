pub struct TokenAuthenticator {
    pub token: String,
}

pub struct ApiUser {
    id: i32,
    name: String, 
}

impl ApiUser {
    pub fn id(&self) -> i32{
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl TokenAuthenticator {
    pub fn new(token: &str) -> Self {
        TokenAuthenticator {
            token: token.to_string(),
        }
    }

    pub fn authenticate(&self) -> Result<bool, String> {

        // sanity checkn
        if self.token.is_empty(){
            // return Error invalid error 
        }


        // it will make an api call, that can return 
        // store it it in ~/.lux/cfg
        // return a simple user adta, that the caller will use to display `welcome $username`

        Ok(false)
    }
}    
