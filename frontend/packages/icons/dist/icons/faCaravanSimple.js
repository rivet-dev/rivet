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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCaravanSimple.js
var require_faCaravanSimple = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCaravanSimple.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "caravan-simple";
    var width = 640;
    var height = 512;
    var aliases = ["caravan-alt"];
    var unicode = "e000";
    var svgPathData = "M80 32C35.8 32 0 67.8 0 112L0 336c0 44.2 35.8 80 80 80l16 0c0 53 43 96 96 96s96-43 96-96l288 0 32 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-32 0 0-160c0-88.4-71.6-160-160-160L80 32zM64 160c0-17.7 14.3-32 32-32l128 0c17.7 0 32 14.3 32 32l0 64c0 17.7-14.3 32-32 32L96 256c-17.7 0-32-14.3-32-32l0-64zm288-32l64 0c17.7 0 32 14.3 32 32l0 64c0 17.7-14.3 32-32 32l-64 0c-17.7 0-32-14.3-32-32l0-64c0-17.7 14.3-32 32-32zM144 416a48 48 0 1 1 96 0 48 48 0 1 1 -96 0z";
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
    exports.faCaravanSimple = exports.definition;
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

// src/_entries/faCaravanSimple.js
var import_faCaravanSimple = __toESM(require_faCaravanSimple());
var export_faCaravanSimple = import_faCaravanSimple.definition;
export {
  export_faCaravanSimple as faCaravanSimple
};
