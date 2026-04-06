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

// src/node_modules/@fortawesome/free-solid-svg-icons/faCancer.js
var require_faCancer = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faCancer.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "cancer";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e847";
    var svgPathData = "M408 152c57.4 0 104 46.6 104 104 0 141.4-114.6 256-256 256-54.3 0-104.8-17-146.3-45.9-14.5-10.1-18-30.1-7.9-44.6s30.1-18 44.6-7.9c31.1 21.7 68.9 34.4 109.7 34.4 67.9 0 127.5-35.3 161.7-88.5-3.2 .3-6.4 .5-9.7 .5-57.4 0-104-46.6-104-104s46.6-104 104-104zM256 0c54.3 0 104.8 17 146.3 45.9 14.5 10.1 18 30.1 7.9 44.6s-30.1 18-44.6 7.9c-31.1-21.7-68.9-34.4-109.7-34.4-67.9 0-127.5 35.2-161.7 88.4 3.2-.3 6.4-.4 9.7-.4 57.4 0 104 46.6 104 104S161.4 360 104 360 0 313.4 0 256C0 254.1 0 252.2 .1 250.4 3.1 111.6 116.5 0 256 0zM104 216a40 40 0 1 0 0 80 40 40 0 1 0 0-80zm304 0a40 40 0 1 0 0 80 40 40 0 1 0 0-80z";
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
    exports.faCancer = exports.definition;
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

// src/_entries/faCancer.js
var import_faCancer = __toESM(require_faCancer());
var export_faCancer = import_faCancer.definition;
export {
  export_faCancer as faCancer
};
