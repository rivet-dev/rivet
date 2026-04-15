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

// src/node_modules/@fortawesome/free-brands-svg-icons/faLumonDrop.js
var require_faLumonDrop = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faLumonDrop.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "lumon-drop";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e7e3";
    var svgPathData = "M480 32c53 0 96 43 96 96l0 224c0 53-43 96-96 96L96 448c-53 0-96-43-96-96L0 128C0 75 43 32 96 32l384 0zM302.4 114.7c-9.8-16.4-20.5-16.4-29.6 0l-68.9 114c-10.7 15.6-16.4 32.8-16.4 52.5 0 50.9 44.3 94.3 100.1 94.3 55 0 100.9-43.5 100.9-94.3 0-19.7-6.6-37.8-17.2-52.5l-68.9-114z";
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
    exports.faLumonDrop = exports.definition;
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

// src/_entries/faLumonDrop.js
var import_faLumonDrop = __toESM(require_faLumonDrop());
var export_faLumonDrop = import_faLumonDrop.definition;
export {
  export_faLumonDrop as faLumonDrop
};
