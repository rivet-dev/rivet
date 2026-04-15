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

// src/node_modules/@fortawesome/free-brands-svg-icons/faUps.js
var require_faUps = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faUps.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "ups";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "f7e0";
    var svgPathData = "M103.2 303c-5.2 3.6-32.6 13.1-32.6-19l0-104-32.7 0 0 102.6c0 74.9 80.2 51.1 97.9 39l0-141.6-32.6 0 0 123zM4 74.8L4 295.7c0 103.7 74.9 135.2 187.7 184.1 112.4-48.9 187.7-80.2 187.7-184.1l0-220.9C263.1 13.2 97.6 25.2 4 74.8zM362.1 295.7c0 86.6-53.2 113.6-170.4 165.3-117.5-51.8-170.5-78.7-170.5-165.3l0-126.4c102.3-93.8 231.6-100 340.9-89.8l0 216.2zM152.5 188.3l0 212.8 32.7 0 0-68.7c24.4 7.3 71.7-2.6 71.7-78.5 0-97.4-80.7-80.9-104.4-65.6zm32.7 117.3l0-100.3c8.4-4.2 38.4-12.7 38.4 49.3 0 67.9-36.4 51.8-38.4 51zm79.1-86.4c.1 47.3 51.6 42.5 52.2 70.4 .6 23.5-30.4 23-50.8 4.9l0 30.1c36.2 21.5 81.9 8.1 83.2-33.5 1.7-51.5-54.1-46.6-53.4-73.2 .6-20.3 30.6-20.5 48.5-2.2l0-28.4c-28.5-22-79.9-9.2-79.7 31.9z";
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
    exports.faUps = exports.definition;
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

// src/_entries/faUps.js
var import_faUps = __toESM(require_faUps());
var export_faUps = import_faUps.definition;
export {
  export_faUps as faUps
};
