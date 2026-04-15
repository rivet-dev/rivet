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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLightEmergencyOn.js
var require_faLightEmergencyOn = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLightEmergencyOn.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "light-emergency-on";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e420";
    var svgPathData = "M69.3 36c-11-7.4-25.9-4.4-33.3 6.7s-4.4 25.9 6.7 33.3l48 32c11 7.4 25.9 4.4 33.3-6.7s4.4-25.9-6.7-33.3l-48-32zM597.3 76c11-7.4 14-22.3 6.7-33.3s-22.3-14-33.3-6.7l-48 32c-11 7.4-14 22.3-6.7 33.3s22.3 14 33.3 6.7l48-32zM24 192c-13.3 0-24 10.7-24 24s10.7 24 24 24l64 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-64 0zm528 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l64 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-64 0zM320 32c-88.4 0-160 71.6-160 160l0 160 320 0 0-160c0-88.4-71.6-160-160-160zM240 192c0 8.8-7.2 16-16 16s-16-7.2-16-16c0-61.9 50.1-112 112-112c8.8 0 16 7.2 16 16s-7.2 16-16 16c-44.2 0-80 35.8-80 80zM96 416l0 32c0 17.7 14.3 32 32 32l384 0c17.7 0 32-14.3 32-32l0-32c0-17.7-14.3-32-32-32l-384 0c-17.7 0-32 14.3-32 32z";
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
    exports.faLightEmergencyOn = exports.definition;
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

// src/_entries/faLightEmergencyOn.js
var import_faLightEmergencyOn = __toESM(require_faLightEmergencyOn());
var export_faLightEmergencyOn = import_faLightEmergencyOn.definition;
export {
  export_faLightEmergencyOn as faLightEmergencyOn
};
