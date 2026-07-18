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

// src/node_modules/@fortawesome/free-solid-svg-icons/faHockeyPuck.js
var require_faHockeyPuck = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faHockeyPuck.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hockey-puck";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f453";
    var svgPathData = "M256 256C114.6 256 0 213 0 160s114.6-96 256-96 256 43 256 96-114.6 96-256 96zM0 352L0 242.7c16.9 12.3 37 22.2 58.1 30.1 53 19.9 123 31.2 197.9 31.2s144.9-11.3 197.9-31.2c21.2-7.9 41.2-17.8 58.1-30.1L512 352c0 53-114.6 96-256 96S0 405 0 352z";
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
    exports.faHockeyPuck = exports.definition;
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

// src/_entries/faHockeyPuck.js
var import_faHockeyPuck = __toESM(require_faHockeyPuck());
var export_faHockeyPuck = import_faHockeyPuck.definition;
export {
  export_faHockeyPuck as faHockeyPuck
};
