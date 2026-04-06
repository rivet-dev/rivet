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

// src/node_modules/@fortawesome/free-brands-svg-icons/faConfluence.js
var require_faConfluence = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faConfluence.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "confluence";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f78d";
    var svgPathData = "M2.8 412.2c-4.5 7.6-2.1 17.5 5.5 22.2l105.9 65.2c7.7 4.7 17.7 2.4 22.4-5.3 0-.1 .1-.2 .1-.2 67.1-112.2 80.5-95.9 280.9-.7 8.1 3.9 17.8 .4 21.7-7.7 .1-.1 .1-.3 .2-.4l50.4-114.1c3.6-8.1-.1-17.6-8.1-21.3-22.2-10.4-66.2-31.2-105.9-50.3-247.9-120.6-330.8 45.7-373.1 112.6zM510.2 100.1c4.5-7.6 2.1-17.5-5.5-22.2L398.9 12.8c-7.5-5-17.6-3.1-22.6 4.4-.2 .3-.4 .6-.6 1-67.3 112.6-81.1 95.6-280.6 .9-8.1-3.9-17.8-.4-21.7 7.7-.1 .1-.1 .3-.2 .4L22.7 141.3c-3.6 8.1 .1 17.6 8.1 21.3 22.2 10.4 66.3 31.2 106 50.4 248 120 330.8-45.4 373.4-112.9z";
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
    exports.faConfluence = exports.definition;
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

// src/_entries/faConfluence.js
var import_faConfluence = __toESM(require_faConfluence());
var export_faConfluence = import_faConfluence.definition;
export {
  export_faConfluence as faConfluence
};
