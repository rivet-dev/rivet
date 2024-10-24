import { ResourceGroup, Resource } from '@/components/Resources';
import { Icon, faRocket, faBooks, faStopwatch, faServer, faCloud, faScrewdriverWrench, faChess, faPartyHorn, faKey, faShareNodes, faRankingStar, faCoinFront, faFloppyDisk, faPuzzle, faFileCode, faDatabase, faBolt, faToolbox } from '@rivet-gg/icons';

export function LearnHome({ engineId, engineName, tutorials }) {
  const prefix = `/docs/${engineId}`;
  return (
    <>
      <ResourceGroup title='Tutorials' columns={2}>
        {tutorials.map(x => (
          <Resource key={x.id} icon={faRocket} title={x.title} href={`${prefix}/tutorials/${x.id}`}>
            <Icon icon={faStopwatch} /> {x.duration}
          </Resource>
        ))}
      </ResourceGroup>

      <ResourceGroup title="Rivet Modules" columns={2}>
        <Resource title="Modules Overview" icon={faPuzzle} href={`${prefix}/general/modules`}>
          {`Use Rivet Modules to add game features.`}
        </Resource>
        <Resource title="Matchmaking & Lobbies" icon={faChess} href={`${prefix}/general/modules/categories/matchmaking`}>
          {`Add matchmaking features to your game.`}
        </Resource>
        {/*<Resource title="Parties" icon={faPartyHorn} href={`${prefix}/general/modules/categories/parties`}>
          {`Implement party systems in your game.`}
        </Resource>*/}
        <Resource title="Authentication" icon={faKey} href={`${prefix}/general/modules/categories/authentication`}>
          {`Add secure authentication to your game.`}
        </Resource>
        <Resource title="Friends, Groups, & Chat" icon={faShareNodes} href={`${prefix}/general/modules/categories/social`}>
          {`Integrate social features into your game.`}
        </Resource>
        {/*<Resource title="Competitive" icon={faRankingStar} href={`${prefix}/general/modules/categories/competitive`}>
          {`Add competitive elements to your game.`}
        </Resource>
        <Resource title="Economy" icon={faCoinFront} href={`${prefix}/general/modules/categories/economy`}>
          {`Implement in-game economy systems.`}
        </Resource>*/}
        <Resource title="Storage" icon={faFloppyDisk} href={`${prefix}/general/modules/categories/storage`}>
          {`Add data storage capabilities to your game.`}
        </Resource>
      </ResourceGroup>

      <ResourceGroup title="Developing Modules" columns={2}>
        <Resource title='Developing Modules Overview' icon={faScrewdriverWrench} href={`/docs/${engineId}/modules/build`}>
          {`Build new functionality with easy-to-use scripting, database, & SDK generation.`}
        </Resource>
        <Resource title="Scripts" icon={faFileCode} href={`${prefix}/general/modules/build/scripts`}>
          {`Create custom scripts for your game logic.`}
        </Resource>
        <Resource title="Database" icon={faDatabase} href={`${prefix}/general/modules/build/database`}>
          {`Set up and manage your game's database.`}
        </Resource>
        <Resource title="Actors" icon={faBolt} href={`${prefix}/general/modules/build/actors`}>
          {`Define and manage game actors and entities.`}
        </Resource>
        <Resource title="Utility Modules" icon={faToolbox} href={`${prefix}/general/modules/build/utility-modules`}>
          {`Access a collection of utility modules for common game development tasks.`}
        </Resource>
      </ResourceGroup>

      <ResourceGroup title="Advanced" columns={2}>
        <Resource title='Dynamic Server' icon={faServer} href={`/docs/${engineId}/general/dynamic-servers`}>
          Low-level API to manage servers & networking.
        </Resource>
        <Resource title='Cloud' icon={faBooks} href={`/docs/${engineId}/general/cloud`}>
          Low-level API to automate deployments & infrastructure.
        </Resource>
      </ResourceGroup>
    </>
  );
}
