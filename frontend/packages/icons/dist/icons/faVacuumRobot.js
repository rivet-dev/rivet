var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/pro-solid-svg-icons/faVacuumRobot.js
var require_faVacuumRobot = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faVacuumRobot.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "vacuum-robot";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e04e";
    var svgPathData = "M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zM356.7 356.7c55.6-55.6 55.6-145.8 0-201.4s-145.8-55.6-201.4 0c-6.2 6.2-16.4 6.2-22.6 0s-6.2-16.4 0-22.6c68.1-68.1 178.5-68.1 246.6 0s68.1 178.5 0 246.6c-6.2 6.2-16.4 6.2-22.6 0s-6.2-16.4 0-22.6zm-144-144c6.2-6.2 16.4-6.2 22.6 0l64 64c6.2 6.2 6.2 16.4 0 22.6s-16.4 6.2-22.6 0l-64-64c-6.2-6.2-6.2-16.4 0-22.6zm-80 16c6.2-6.2 16.4-6.2 22.6 0l128 128c6.2 6.2 6.2 16.4 0 22.6s-16.4 6.2-22.6 0l-128-128c-6.2-6.2-6.2-16.4 0-22.6zm-16 102.6c-6.2-6.2-6.2-16.4 0-22.6s16.4-6.2 22.6 0l64 64c6.2 6.2 6.2 16.4 0 22.6s-16.4 6.2-22.6 0l-64-64z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faVacuumRobot = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faVacuumRobot.js
var import_faVacuumRobot = __toESM(require_faVacuumRobot());
var export_faVacuumRobot = import_faVacuumRobot.definition;
export {
  export_faVacuumRobot as faVacuumRobot
};
