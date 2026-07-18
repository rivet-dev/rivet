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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUserRobot.js
var require_faUserRobot = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUserRobot.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-robot";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e04b";
    var svgPathData = "M224 0c8.8 0 16 7.2 16 16l0 48 64 0c35.3 0 64 28.7 64 64l0 128c0 35.3-28.7 64-64 64l-160 0c-35.3 0-64-28.7-64-64l0-128c0-35.3 28.7-64 64-64l64 0 0-48c0-8.8 7.2-16 16-16zM16 144c0-8.8 7.2-16 16-16s16 7.2 16 16l0 96c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-96zm400-16c8.8 0 16 7.2 16 16l0 96c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-96c0-8.8 7.2-16 16-16zM160 192a32 32 0 1 0 0-64 32 32 0 1 0 0 64zm160-32a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zM160 240a16 16 0 1 0 0 32 16 16 0 1 0 0-32zm48 16a16 16 0 1 0 32 0 16 16 0 1 0 -32 0zm80-16a16 16 0 1 0 0 32 16 16 0 1 0 0-32zM96 352l256 0c53 0 96 43 96 96l0 32c0 17.7-14.3 32-32 32l-64 0 0-64c0-17.7-14.3-32-32-32l-192 0c-17.7 0-32 14.3-32 32l0 64-64 0c-17.7 0-32-14.3-32-32l0-32c0-53 43-96 96-96zm80 112l0 48-32 0 0-48c0-8.8 7.2-16 16-16s16 7.2 16 16zm48-16c8.8 0 16 7.2 16 16l0 48-32 0 0-48c0-8.8 7.2-16 16-16zm80 16l0 48-32 0 0-48c0-8.8 7.2-16 16-16s16 7.2 16 16z";
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
    exports.faUserRobot = exports.definition;
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

// src/_entries/faUserRobot.js
var import_faUserRobot = __toESM(require_faUserRobot());
var export_faUserRobot = import_faUserRobot.definition;
export {
  export_faUserRobot as faUserRobot
};
