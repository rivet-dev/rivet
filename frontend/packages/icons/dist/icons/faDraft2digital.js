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

// src/node_modules/@fortawesome/free-brands-svg-icons/faDraft2digital.js
var require_faDraft2digital = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faDraft2digital.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "draft2digital";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f396";
    var svgPathData = "M496 398.1l-144-82.2 0 64.7-91.3 0c30.8-35 81.8-95.9 111.8-149.3 35.2-62.6 16.1-123.4-12.8-153.3-4.4-4.6-62.2-62.9-166-41.2-59.1 12.4-89.4 43.4-104.3 67.3-13.1 20.9-17 39.8-18.2 47.7-5.5 33 19.4 67.1 56.7 67.1 31.7 0 57.3-25.7 57.3-57.4 0-27.1-19.7-52.1-48-56.8 1.8-7.3 17.7-21.1 26.3-24.7 41.1-17.3 78 5.2 83.3 33.5 8.3 44.3-37.1 90.4-69.7 127.6-76.6 87-142.8 155.7-161.1 174.8l336-.1 0 64.2 144-81.9zM385.9 371l47.1 27.2-47.1 27.2 0-54.4zM105.4 161.4a22.4 22.4 0 1 1 44.8 0 22.4 22.4 0 1 1 -44.8 0zM98.5 380.5c25.6-27.4 97.7-104.7 150.8-169.9 35.1-43.1 40.3-82.4 28.4-112.7-7.4-18.8-17.5-30.2-24.3-35.7 45.3 2.1 68 23.4 82.2 38.3 0 0 42.4 48.2 5.8 113.3-37 65.9-110.9 147.5-128.5 166.7l-114.4 0z";
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
    exports.faDraft2digital = exports.definition;
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

// src/_entries/faDraft2digital.js
var import_faDraft2digital = __toESM(require_faDraft2digital());
var export_faDraft2digital = import_faDraft2digital.definition;
export {
  export_faDraft2digital as faDraft2digital
};
