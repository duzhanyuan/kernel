(function() {var implementors = {};
implementors["arrayvec"] = [];
implementors["paging"] = [];
implementors["sos_alloc"] = [];
implementors["sos_kernel"] = [];

            if (window.register_implementors) {
                window.register_implementors(implementors);
            } else {
                window.pending_implementors = implementors;
            }
        
})()