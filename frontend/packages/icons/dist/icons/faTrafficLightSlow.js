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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTrafficLightSlow.js
var require_faTrafficLightSlow = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTrafficLightSlow.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "traffic-light-slow";
    var width = 320;
    var height = 512;
    var aliases = [];
    var unicode = "f639";
    var svgPathData = "M64 0C28.7 0 0 28.7 0 64L0 352c0 88.4 71.6 160 160 160s160-71.6 160-160l0-288c0-35.3-28.7-64-64-64L64 0zm72 368a24 24 0 1 0 48 0 24 24 0 1 0 -48 0zm72 0a48 48 0 1 1 -96 0 48 48 0 1 1 96 0zm0-128a48 48 0 1 1 -96 0 48 48 0 1 1 96 0zM136 112a24 24 0 1 0 48 0 24 24 0 1 0 -48 0zm72 0a48 48 0 1 1 -96 0 48 48 0 1 1 96 0z";
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
    exports.faTrafficLightSlow = exports.definition;
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

// src/_entries/faTrafficLightSlow.js
var import_faTrafficLightSlow = __toESM(require_faTrafficLightSlow());
var export_faTrafficLightSlow = import_faTrafficLightSlow.definition;
export {
  export_faTrafficLightSlow as faTrafficLightSlow
};
