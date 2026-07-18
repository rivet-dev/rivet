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

// src/node_modules/@fortawesome/free-brands-svg-icons/faMeta.js
var require_faMeta = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faMeta.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "meta";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e49b";
    var svgPathData = "M640 317.9c0 91.3-39.4 148.5-110.3 148.5-62.6 0-95.8-34.6-156.9-136.6l-31.4-52.6c-8.3-12.5-14.5-24.2-21.2-35-20.1 33.8-47.1 83-47.1 83-67 116.6-104.6 141.2-156.9 141.2-72.8 0-116.2-57.3-116.2-145.9 0-143 79.8-278.1 183.9-278.1 50.2 0 93.8 24.7 144.8 89.5 37.1-50.1 78.1-89.5 130.6-89.5 99.1 0 180.7 125.7 180.7 275.5zM287.4 192.2c-42.9-62.1-70.9-80.5-104.4-80.5-61.9 0-113.8 106.1-113.8 210 0 48.5 18.5 75.7 49.6 75.7 30.2 0 49-19 103.2-103.8 0 0 24.7-39.1 65.4-101.4zM531.2 397.4c32.2 0 46.9-27.5 46.9-74.9 0-124.2-54.3-225.4-123.2-225.4-33.2 0-61.1 25.9-94.9 78 9.4 13.8 19.1 29 29.3 45.4l37.5 62.4c58.7 94.1 73.5 114.5 104.4 114.5z";
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
    exports.faMeta = exports.definition;
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

// src/_entries/faMeta.js
var import_faMeta = __toESM(require_faMeta());
var export_faMeta = import_faMeta.definition;
export {
  export_faMeta as faMeta
};
