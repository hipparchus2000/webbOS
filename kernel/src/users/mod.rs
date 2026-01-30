//! User Management System
//!
//! Multi-user support for WebbOS with authentication and permissions.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use alloc::collections::BTreeMap;
use spin::Mutex;
use lazy_static::lazy_static;

use crate::println;
use crate::crypto::sha256;

/// User ID type
pub type UserId = u32;

/// Group ID type
pub type GroupId = u32;

/// User account
#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub password_hash: [u8; 32], // SHA-256 hash
    pub home_directory: String,
    pub shell: String,
    pub groups: Vec<GroupId>,
    pub is_admin: bool,
    pub is_active: bool,
}

/// User group
#[derive(Debug, Clone)]
pub struct Group {
    pub id: GroupId,
    pub name: String,
    pub members: Vec<UserId>,
}

/// Session for logged-in user
#[derive(Debug, Clone)]
pub struct Session {
    pub session_id: u64,
    pub user_id: UserId,
    pub start_time: u64, // Unix timestamp
}

/// User manager
pub struct UserManager {
    users: BTreeMap<UserId, User>,
    groups: BTreeMap<GroupId, Group>,
    sessions: BTreeMap<u64, Session>,
    next_user_id: UserId,
    next_group_id: GroupId,
    next_session_id: u64,
    current_user: Option<UserId>,
}

impl UserManager {
    /// Create new user manager
    fn new() -> Self {
        let mut manager = Self {
            users: BTreeMap::new(),
            groups: BTreeMap::new(),
            sessions: BTreeMap::new(),
            next_user_id: 1000,
            next_group_id: 1000,
            next_session_id: 1,
            current_user: None,
        };
        
        // Create default admin user
        manager.create_user_internal(
            "admin",
            "admin",
            "/home/admin",
            "/bin/shell",
            true,
        );
        
        // Create default regular user
        manager.create_user_internal(
            "user",
            "user",
            "/home/user",
            "/bin/shell",
            false,
        );
        
        manager
    }
    
    /// Create a new user (internal)
    fn create_user_internal(
        &mut self,
        username: &str,
        password: &str,
        home: &str,
        shell: &str,
        is_admin: bool,
    ) -> UserId {
        let id = self.next_user_id;
        self.next_user_id += 1;
        
        let password_hash = hash_password(password);
        
        let user = User {
            id,
            username: String::from(username),
            password_hash,
            home_directory: String::from(home),
            shell: String::from(shell),
            groups: Vec::new(),
            is_admin,
            is_active: true,
        };
        
        self.users.insert(id, user);
        id
    }
    
    /// Create a new user (public API)
    pub fn create_user(
        &mut self,
        username: &str,
        password: &str,
        is_admin: bool,
    ) -> Result<UserId, UserError> {
        // Check if username already exists
        if self.find_user_by_name(username).is_some() {
            return Err(UserError::UsernameExists);
        }
        
        // Validate username
        if username.is_empty() || username.len() > 32 {
            return Err(UserError::InvalidUsername);
        }
        
        // Validate password
        if password.len() < 4 {
            return Err(UserError::WeakPassword);
        }
        
        let home = format!("/home/{}", username);
        let id = self.create_user_internal(username, password, &home, "/bin/shell", is_admin);
        
        println!("[users] Created user '{}' with ID {}", username, id);
        Ok(id)
    }
    
    /// Authenticate user
    pub fn authenticate(&mut self, username: &str, password: &str) -> Option<UserId> {
        let password_hash = hash_password(password);
        
        for (id, user) in &self.users {
            if user.username == username 
                && user.password_hash == password_hash
                && user.is_active {
                return Some(*id);
            }
        }
        
        None
    }
    
    /// Login user and create session
    pub fn login(&mut self, username: &str, password: &str) -> Option<u64> {
        if let Some(user_id) = self.authenticate(username, password) {
            let session_id = self.next_session_id;
            self.next_session_id += 1;
            
            let session = Session {
                session_id,
                user_id,
                start_time: get_current_time(),
            };
            
            self.sessions.insert(session_id, session);
            self.current_user = Some(user_id);
            
            println!("[users] User '{}' logged in (session {})", username, session_id);
            Some(session_id)
        } else {
            None
        }
    }
    
    /// Logout user
    pub fn logout(&mut self, session_id: u64) -> bool {
        if let Some(session) = self.sessions.remove(&session_id) {
            if let Some(user) = self.users.get(&session.user_id) {
                println!("[users] User '{}' logged out", user.username);
            }
            
            if self.sessions.is_empty() {
                self.current_user = None;
            }
            
            true
        } else {
            false
        }
    }
    
    /// Get current user
    pub fn current_user(&self) -> Option<&User> {
        self.current_user.and_then(|id| self.users.get(&id))
    }
    
    /// Get user by ID
    pub fn get_user(&self, id: UserId) -> Option<&User> {
        self.users.get(&id)
    }
    
    /// Find user by name
    pub fn find_user_by_name(&self, username: &str) -> Option<&User> {
        self.users.values().find(|u| u.username == username)
    }
    
    /// Get all users
    pub fn list_users(&self) -> Vec<&User> {
        self.users.values().collect()
    }
    
    /// Change password
    pub fn change_password(&mut self, user_id: UserId, new_password: &str) -> Result<(), UserError> {
        if new_password.len() < 4 {
            return Err(UserError::WeakPassword);
        }
        
        if let Some(user) = self.users.get_mut(&user_id) {
            user.password_hash = hash_password(new_password);
            println!("[users] Password changed for user '{}'", user.username);
            Ok(())
        } else {
            Err(UserError::UserNotFound)
        }
    }
    
    /// Delete user
    pub fn delete_user(&mut self, user_id: UserId) -> Result<(), UserError> {
        // Prevent deleting the last admin
        if let Some(user) = self.users.get(&user_id) {
            if user.is_admin {
                let admin_count = self.users.values().filter(|u| u.is_admin).count();
                if admin_count <= 1 {
                    return Err(UserError::CannotDeleteLastAdmin);
                }
            }
        }
        
        if let Some(user) = self.users.remove(&user_id) {
            // End all sessions for this user
            self.sessions.retain(|_, s| s.user_id != user_id);
            println!("[users] Deleted user '{}'", user.username);
            Ok(())
        } else {
            Err(UserError::UserNotFound)
        }
    }
    
    /// Set user active/inactive
    pub fn set_user_active(&mut self, user_id: UserId, active: bool) -> Result<(), UserError> {
        if let Some(user) = self.users.get_mut(&user_id) {
            user.is_active = active;
            println!("[users] User '{}' {}", user.username, 
                if active { "activated" } else { "deactivated" });
            Ok(())
        } else {
            Err(UserError::UserNotFound)
        }
    }
    
    /// Get active sessions
    pub fn list_sessions(&self) -> Vec<&Session> {
        self.sessions.values().collect()
    }
    
    /// Get session info
    pub fn get_session(&self, session_id: u64) -> Option<&Session> {
        self.sessions.get(&session_id)
    }
}

/// User errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserError {
    UserNotFound,
    UsernameExists,
    InvalidUsername,
    WeakPassword,
    CannotDeleteLastAdmin,
    NotAuthenticated,
}

/// Global user manager
lazy_static! {
    static ref USER_MANAGER: Mutex<UserManager> = Mutex::new(UserManager::new());
}

/// Hash password using SHA-256
fn hash_password(password: &str) -> [u8; 32] {
    let mut hasher = sha256::Sha256::new();
    hasher.update(password.as_bytes());
    // Add a simple salt
    hasher.update(b"WebbOS");
    hasher.finalize()
}

/// Get current time (placeholder)
fn get_current_time() -> u64 {
    // TODO: Implement real time
    0
}

/// Initialize user system
pub fn init() {
    println!("[users] Initializing user management system...");
    
    let manager = USER_MANAGER.lock();
    println!("[users] {} users configured", manager.users.len());
    
    // List default users
    for user in manager.list_users() {
        println!("[users]   - {} ({})", 
            user.username,
            if user.is_admin { "admin" } else { "user" }
        );
    }
}

/// Login user
pub fn login(username: &str, password: &str) -> Option<u64> {
    USER_MANAGER.lock().login(username, password)
}

/// Logout user
pub fn logout(session_id: u64) -> bool {
    USER_MANAGER.lock().logout(session_id)
}

/// Get current user
pub fn current_user() -> Option<User> {
    USER_MANAGER.lock().current_user().cloned()
}

/// Create new user (requires admin)
pub fn create_user(username: &str, password: &str, is_admin: bool) -> Result<UserId, UserError> {
    USER_MANAGER.lock().create_user(username, password, is_admin)
}

/// List all users
pub fn list_users() -> Vec<User> {
    USER_MANAGER.lock().list_users().into_iter().cloned().collect()
}

/// Delete user
pub fn delete_user(user_id: UserId) -> Result<(), UserError> {
    USER_MANAGER.lock().delete_user(user_id)
}

/// Change password
pub fn change_password(user_id: UserId, new_password: &str) -> Result<(), UserError> {
    USER_MANAGER.lock().change_password(user_id, new_password)
}

/// Print user info
pub fn print_users() {
    println!("\nUser Accounts:");
    println!("{:<6} {:<16} {:<10} {:<12} {}", "ID", "Username", "Type", "Status", "Home");
    println!("{:-<70}", "");
    
    for user in list_users() {
        println!("{:<6} {:<16} {:<10} {:<12} {}",
            user.id,
            user.username,
            if user.is_admin { "admin" } else { "user" },
            if user.is_active { "active" } else { "inactive" },
            user.home_directory
        );
    }
}

/// Print sessions
pub fn print_sessions() {
    let manager = USER_MANAGER.lock();
    let sessions = manager.list_sessions();
    
    println!("\nActive Sessions:");
    println!("{:<12} {:<8} {:<16} {}", "Session ID", "User ID", "Username", "Start Time");
    println!("{:-<60}", "");
    
    for session in sessions {
        if let Some(user) = manager.get_user(session.user_id) {
            println!("{:<12} {:<8} {:<16} {}",
                session.session_id,
                session.user_id,
                user.username,
                session.start_time
            );
        }
    }
}
