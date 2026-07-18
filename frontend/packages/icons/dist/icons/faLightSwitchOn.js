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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLightSwitchOn.js
var require_faLightSwitchOn = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLightSwitchOn.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "light-switch-on";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e019";
    var svgPathData = "M64 0C28.7 0 0 28.7 0 64L0 448c0 35.3 28.7 64 64 64l256 0c35.3 0 64-28.7 64-64l0-384c0-35.3-28.7-64-64-64L64 0zm96 96l64 0c35.3 0 64 28.7 64 64l0 192c0 35.3-28.7 64-64 64l-64 0c-35.3 0-64-28.7-64-64l0-192c0-35.3 28.7-64 64-64zm32-64a16 16 0 1 1 0 32 16 16 0 1 1 0-32zm0 416a16 16 0 1 1 0 32 16 16 0 1 1 0-32zM160 128c-17.7 0-32 14.3-32 32l0 96 128 0 0-96c0-17.7-14.3-32-32-32l-64 0z";
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
    exports.faLightSwitchOn = exports.definition;
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

// src/_entries/faLightSwitchOn.js
var import_faLightSwitchOn = __toESM(require_faLightSwitchOn());
var export_faLightSwitchOn = import_faLightSwitchOn.definition;
export {
  export_faLightSwitchOn as faLightSwitchOn
};
