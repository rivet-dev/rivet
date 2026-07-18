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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faRobotAstromech.js
var require_faRobotAstromech = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faRobotAstromech.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "robot-astromech";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e2d2";
    var svgPathData = "M256 0c70.7 0 128 57.3 128 128l0 32-256 0 0-32C128 57.3 185.3 0 256 0zm0 96a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zm48 32a16 16 0 1 0 0-32 16 16 0 1 0 0 32zM48 240c0-26.5 21.5-48 48-48l320 0c26.5 0 48 21.5 48 48l0 158.3c0 8.7 2.4 17.2 6.8 24.7l38.9 64.8c3 4.9 3 11.1 .2 16.1s-8.2 8.1-13.9 8.1l-96 0c-8.8 0-16-7.2-16-16l0-112-45.3 45.3c-12 12-28.3 18.7-45.3 18.7l-75 0c-17 0-33.3-6.7-45.3-18.7L128 384l0 112c0 8.8-7.2 16-16 16l-96 0c-5.8 0-11.1-3.1-13.9-8.1s-2.8-11.2 .2-16.1L41.2 423c4.5-7.5 6.8-16 6.8-24.7L48 240zm160 16c-8.8 0-16 7.2-16 16s7.2 16 16 16l96 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-96 0zm0 64c-8.8 0-16 7.2-16 16s7.2 16 16 16l96 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-96 0z";
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
    exports.faRobotAstromech = exports.definition;
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

// src/_entries/faRobotAstromech.js
var import_faRobotAstromech = __toESM(require_faRobotAstromech());
var export_faRobotAstromech = import_faRobotAstromech.definition;
export {
  export_faRobotAstromech as faRobotAstromech
};
