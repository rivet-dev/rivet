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

// src/node_modules/@fortawesome/free-solid-svg-icons/faWheelchair.js
var require_faWheelchair = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faWheelchair.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "wheelchair";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f193";
    var svgPathData = "M136 40a56 56 0 1 1 112 0 56 56 0 1 1 -112 0zM117.9 197.6l12.6 63c-38.6 12.4-66.5 48.7-66.5 91.4 0 53 43 96 96 96 35.6 0 66.7-19.4 83.3-48.2 2.3 .1 4.6 .2 7 .2l4.1 0c1 0 2.1 0 3.1 0l55.2 0c-20.4 64.9-81 112-152.7 112-88.4 0-160-71.6-160-160 0-73.8 50-135.9 117.9-154.4zm148.6-28.1l23.7 118.5 60.5 0c33.3 0 63.2 20.7 74.9 51.9l25.5 68 18.7-6.2c16.8-5.6 34.9 3.5 40.5 20.2s-3.5 34.9-20.2 40.5l-48 16c-16.3 5.4-34-3-40.1-19.1l-36.3-96.9c-2.3-6.2-8.3-10.4-15-10.4l-86.1 0c-.4 0-.8 0-1.3 0l-13.1 0c-30.5 0-56.8-21.5-62.8-51.4L165.3 189.7c-6.4-31.9 18-61.7 50.6-61.7 24.6 0 45.8 17.4 50.6 41.5z";
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
    exports.faWheelchair = exports.definition;
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

// src/_entries/faWheelchair.js
var import_faWheelchair = __toESM(require_faWheelchair());
var export_faWheelchair = import_faWheelchair.definition;
export {
  export_faWheelchair as faWheelchair
};
