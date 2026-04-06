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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLightEmergency.js
var require_faLightEmergency = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLightEmergency.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "light-emergency";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e41f";
    var svgPathData = "M64 192c0-88.4 71.6-160 160-160s160 71.6 160 160l0 160L64 352l0-160zm160-80c8.8 0 16-7.2 16-16s-7.2-16-16-16c-61.9 0-112 50.1-112 112c0 8.8 7.2 16 16 16s16-7.2 16-16c0-44.2 35.8-80 80-80zM32 384l384 0c17.7 0 32 14.3 32 32l0 32c0 17.7-14.3 32-32 32L32 480c-17.7 0-32-14.3-32-32l0-32c0-17.7 14.3-32 32-32z";
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
    exports.faLightEmergency = exports.definition;
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

// src/_entries/faLightEmergency.js
var import_faLightEmergency = __toESM(require_faLightEmergency());
var export_faLightEmergency = import_faLightEmergency.definition;
export {
  export_faLightEmergency as faLightEmergency
};
