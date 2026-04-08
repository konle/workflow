// MongoDB migration script: fix lowercase status values → PascalCase
// Usage: mongosh workflow scripts/fix_status_casing.js
//   or:  mongosh --host <host> --port <port> workflow scripts/fix_status_casing.js

const dryRun = typeof _dryRun !== "undefined" ? _dryRun : false;

function fixField(collection, field, mapping) {
  let totalMatched = 0;
  let totalModified = 0;

  for (const [wrong, correct] of Object.entries(mapping)) {
    const filter = { [field]: wrong };
    const count = db.getCollection(collection).countDocuments(filter);
    if (count === 0) continue;

    print(`  [${collection}] ${field}: "${wrong}" → "${correct}"  (${count} docs)`);
    totalMatched += count;

    if (!dryRun) {
      const res = db.getCollection(collection).updateMany(filter, { $set: { [field]: correct } });
      totalModified += res.modifiedCount;
    }
  }
  return { totalMatched, totalModified };
}

// lowercase → PascalCase mappings
const workflowStatusMap = {
  draft: "Draft",
  published: "Published",
  deleted: "Deleted",
  archived: "Archived",
};

const instanceStatusMap = {
  pending: "Pending",
  running: "Running",
  await: "Await",
  completed: "Completed",
  failed: "Failed",
  canceled: "Canceled",
  suspended: "Suspended",
};

const taskInstanceStatusMap = {
  pending: "Pending",
  running: "Running",
  completed: "Completed",
  failed: "Failed",
  canceled: "Canceled",
};

const taskStatusMap = {
  draft: "Draft",
  published: "Published",
};

print("=== Status Casing Migration" + (dryRun ? " [DRY RUN]" : "") + " ===\n");

// 1. workflow_entities.status
print("1. workflow_entities.status (WorkflowStatus)");
fixField("workflow_entities", "status", workflowStatusMap);

// 2. workflow_meta_entities.status (same WorkflowStatus)
print("\n2. workflow_meta_entities.status (WorkflowStatus)");
fixField("workflow_meta_entities", "status", workflowStatusMap);

// 3. workflow_instances.status (WorkflowInstanceStatus)
print("\n3. workflow_instances.status (WorkflowInstanceStatus)");
fixField("workflow_instances", "status", instanceStatusMap);

// 4. workflow_instances.nodes[].status (nested node status = WorkflowInstanceStatus)
print("\n4. workflow_instances nested nodes[].status");
let nestedNodeFixed = 0;
for (const [wrong, correct] of Object.entries(instanceStatusMap)) {
  const filter = { "nodes.status": wrong };
  const count = db.workflow_instances.countDocuments(filter);
  if (count === 0) continue;
  print(`  [workflow_instances] nodes.$.status: "${wrong}" → "${correct}"  (${count} docs)`);
  if (!dryRun) {
    let keepGoing = true;
    while (keepGoing) {
      const res = db.workflow_instances.updateMany(
        { "nodes.status": wrong },
        { $set: { "nodes.$[elem].status": correct } },
        { arrayFilters: [{ "elem.status": wrong }] }
      );
      nestedNodeFixed += res.modifiedCount;
      keepGoing = res.modifiedCount > 0 && db.workflow_instances.countDocuments({ "nodes.status": wrong }) > 0;
    }
  }
}

// 5. workflow_instances.nodes[].task_instance.task_status (nested TaskInstanceStatus)
print("\n5. workflow_instances nested nodes[].task_instance.task_status");
let nestedTaskFixed = 0;
for (const [wrong, correct] of Object.entries(taskInstanceStatusMap)) {
  const filter = { "nodes.task_instance.task_status": wrong };
  const count = db.workflow_instances.countDocuments(filter);
  if (count === 0) continue;
  print(`  [workflow_instances] nodes.$.task_instance.task_status: "${wrong}" → "${correct}"  (${count} docs)`);
  if (!dryRun) {
    let keepGoing = true;
    while (keepGoing) {
      const res = db.workflow_instances.updateMany(
        { "nodes.task_instance.task_status": wrong },
        { $set: { "nodes.$[elem].task_instance.task_status": correct } },
        { arrayFilters: [{ "elem.task_instance.task_status": wrong }] }
      );
      nestedTaskFixed += res.modifiedCount;
      keepGoing = res.modifiedCount > 0 && db.workflow_instances.countDocuments({ "nodes.task_instance.task_status": wrong }) > 0;
    }
  }
}

// 6. task_instances.task_status (TaskInstanceStatus)
print("\n6. task_instances.task_status (TaskInstanceStatus)");
fixField("task_instances", "task_status", taskInstanceStatusMap);

// 7. tasks.status (TaskStatus)
print("\n7. tasks.status (TaskStatus)");
fixField("tasks", "status", taskStatusMap);

print("\n=== Migration " + (dryRun ? "preview" : "complete") + " ===");
