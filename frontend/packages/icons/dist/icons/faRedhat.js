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

// src/node_modules/@fortawesome/free-brands-svg-icons/faRedhat.js
var require_faRedhat = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faRedhat.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "redhat";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f7bc";
    var svgPathData = "M341.5 285.6c33.7 0 82.3-6.9 82.3-47 .2-6.7 .9-1.8-20.9-96.2-4.6-19.1-8.7-27.8-42.3-44.6-26.1-13.3-82.9-35.4-99.7-35.4-15.7 0-20.2 20.2-38.9 20.2-18 0-31.3-15.1-48.1-15.1-16.1 0-26.7 11-34.8 33.6-27.5 77.6-26.3 74.3-26.1 78.3 0 24.8 97.6 106.1 228.5 106.1zM429 254.8c4.7 22 4.7 24.3 4.7 27.2 0 37.7-42.3 58.6-98 58.6-125.7 .1-235.9-73.6-235.9-122.3 0-6.8 1.4-13.5 4.1-19.7-45.2 2.3-103.8 10.3-103.8 62 0 84.7 200.6 189 359.5 189 121.8 0 152.5-55.1 152.5-98.6 0-34.2-29.6-73-82.9-96.2z";
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
    exports.faRedhat = exports.definition;
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

// src/_entries/faRedhat.js
var import_faRedhat = __toESM(require_faRedhat());
var export_faRedhat = import_faRedhat.definition;
export {
  export_faRedhat as faRedhat
};
