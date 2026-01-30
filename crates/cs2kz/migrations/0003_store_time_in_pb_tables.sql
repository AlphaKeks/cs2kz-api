ALTER TABLE BestNubRecords ADD COLUMN IF NOT EXISTS `time` FLOAT8 NOT NULL;
ALTER TABLE BestProRecords ADD COLUMN IF NOT EXISTS `time` FLOAT8 NOT NULL;

INSERT INTO BestNubRecords (filter_id, player_id, record_id, `time`, points)
SELECT bnr.filter_id, bnr.player_id, bnr.record_id, r.`time`, bnr.points
FROM BestNubRecords AS bnr
INNER JOIN Records AS r ON r.id = bnr.record_id
ON DUPLICATE KEY UPDATE `time` = VALUES(`time`);

INSERT INTO BestProRecords (filter_id, player_id, record_id, `time`, points)
SELECT bpr.filter_id, bpr.player_id, bpr.record_id, r.`time`, bpr.points
FROM BestProRecords AS bpr
INNER JOIN Records AS r ON r.id = bpr.record_id
ON DUPLICATE KEY UPDATE `time` = VALUES(`time`);
