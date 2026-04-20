There are Todos which are missing features and not implemented properly.

1. Naming
Currently naming not working even though i set naming for Employee doctype as byfieldanme: fieldname is employee_name
but still upon saving doc I get name like this new-312312, You need to ensure our naming match with frappe,
2, you will run a full naming test, where you will make ammendments in Employee doctype rule, from Naming Series to by filed_name
each naming type should be through tested.
Important note here: Currently naming is being handled in Rust which is anti pattern for our system, because Naming is purely a db related affair and should be handled by Surreal via Functions,. Generally naming should run as part of our pipeline as a mandotry surreal function and it should be shown in our doc designer too. the naming function of surreal will generall be the standard default one which will be shared by all docs,
however we need to keep this option where we have to override this go on with our own thing outside of framework, Naming handling should be removed from rust totally.  

3. UI Issue-implement commnad pallete 
Why Employee is not showing up in OUr COmmand Pallete, wire it up properly just like frappe
so i can create employee from that command pallete too.

4. UI Issue- List doesnt auto update, it should update simialr to nature in frappe, you need to format our 
list too in syncfusion grid and remove unnessary tools from this so it is better aligned with frappe feel.

5. End to End permissions check, Permissions are very critical area, in our case RBAC of a user is just a 
graph query, currently I belive we are using tabDocPermissions, which i am very skpetical, because I dint think so
we even need it infact such table is not required at all. You will run end to end Permissions Tests
and ensure its better aligned with graph, each user permission and retireval should be a graph 
You will create a new user in System with read only accessand add that user in employee, and try in various scenarios
whether your test gets passed.

You will complete this step by step, each fisrt be complete successfully before you move on the next,
for UI related tasks these can be started in paralell and you can use sub agents here to do the work