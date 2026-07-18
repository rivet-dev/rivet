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

// src/node_modules/@fortawesome/free-solid-svg-icons/faVirus.js
var require_faVirus = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faVirus.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "virus";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e074";
    var svgPathData = "M296 40c0-22.1-17.9-40-40-40s-40 17.9-40 40c0 44.1-53.3 66.1-84.5 35-15.6-15.6-40.9-15.6-56.6 0s-15.6 40.9 0 56.6c31.2 31.2 9.1 84.5-35 84.5-22.1 0-40 17.9-40 40s17.9 40 40 40c44.1 0 66.1 53.3 35 84.5-15.6 15.6-15.6 40.9 0 56.6s40.9 15.6 56.6 0c31.2-31.2 84.5-9.1 84.5 35 0 22.1 17.9 40 40 40s40-17.9 40-40c0-44.1 53.3-66.1 84.5-35 15.6 15.6 40.9 15.6 56.6 0s15.6-40.9 0-56.6c-31.2-31.2-9.1-84.5 35-84.5 22.1 0 40-17.9 40-40s-17.9-40-40-40c-44.1 0-66.1-53.3-35-84.5 15.6-15.6 15.6-40.9 0-56.6s-40.9-15.6-56.6 0C349.3 106.1 296 84.1 296 40zM160 224a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zm160 32a32 32 0 1 1 0 64 32 32 0 1 1 0-64z";
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
    exports.faVirus = exports.definition;
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

// src/_entries/faVirus.js
var import_faVirus = __toESM(require_faVirus());
var export_faVirus = import_faVirus.definition;
export {
  export_faVirus as faVirus
};
