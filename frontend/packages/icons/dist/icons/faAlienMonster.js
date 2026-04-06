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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faAlien8bit.js
var require_faAlien8bit = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faAlien8bit.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "alien-8bit";
    var width = 576;
    var height = 512;
    var aliases = [128126, "alien-monster"];
    var unicode = "f8f6";
    var svgPathData = "M96 48c0-8.8 7.2-16 16-16l32 0c8.8 0 16 7.2 16 16l0 16 48 0c8.8 0 16 7.2 16 16l0 48 128 0 0-48c0-8.8 7.2-16 16-16l48 0 0-16c0-8.8 7.2-16 16-16l32 0c8.8 0 16 7.2 16 16l0 32c0 8.8-7.2 16-16 16l-48 0 0 32 0 32 48 0c8.8 0 16 7.2 16 16l0 48 32 0 0-80c0-8.8 7.2-16 16-16l32 0c8.8 0 16 7.2 16 16l0 128c0 8.8-7.2 16-16 16l-48 0 0 80c0 8.8-7.2 16-16 16l-48 0 0 80c0 8.8-7.2 16-16 16l-48 0-48 0c-8.8 0-16-7.2-16-16l0-32c0-8.8 7.2-16 16-16l48 0 0-32-192 0 0 32 48 0c8.8 0 16 7.2 16 16l0 32c0 8.8-7.2 16-16 16l-48 0-48 0c-8.8 0-16-7.2-16-16l0-80-48 0c-8.8 0-16-7.2-16-16l0-80-48 0c-8.8 0-16-7.2-16-16L0 144c0-8.8 7.2-16 16-16l32 0c8.8 0 16 7.2 16 16l0 80 32 0 0-48c0-8.8 7.2-16 16-16l48 0 0-32 0-32-48 0c-8.8 0-16-7.2-16-16l0-32zm64 192l0 64c0 8.8 7.2 16 16 16l32 0c8.8 0 16-7.2 16-16l0-64c0-8.8-7.2-16-16-16l-32 0c-8.8 0-16 7.2-16 16zm192 0l0 64c0 8.8 7.2 16 16 16l32 0c8.8 0 16-7.2 16-16l0-64c0-8.8-7.2-16-16-16l-32 0c-8.8 0-16 7.2-16 16z";
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
    exports.faAlien8bit = exports.definition;
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

// src/_entries/faAlienMonster.js
var import_faAlien8bit = __toESM(require_faAlien8bit());
var export_faAlienMonster = import_faAlien8bit.definition;
export {
  export_faAlienMonster as faAlienMonster
};
