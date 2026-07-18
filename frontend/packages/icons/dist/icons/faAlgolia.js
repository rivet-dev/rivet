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

// src/node_modules/@fortawesome/free-brands-svg-icons/faAlgolia.js
var require_faAlgolia = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faAlgolia.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "algolia";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f36c";
    var svgPathData = "M256 0c-139.9 0-254 112.7-256 252.1-2 141.5 112.9 258.7 254.5 259.5 43.7 .3 85.9-10.4 123.3-30.7 3.6-2 4.2-7 1.1-9.7l-24-21.2c-4.9-4.3-11.8-5.5-17.8-3-26.1 11.1-54.5 16.8-83.7 16.4-114.4-1.5-206.9-96.6-205.1-211 1.8-112.9 94.3-204.2 207.7-204.2l207.7 0 0 369-117.8-104.7c-3.8-3.4-9.7-2.7-12.7 1.3-18.9 25-49.7 40.6-83.9 38.2-47.5-3.3-85.9-41.5-89.5-88.9-4.2-56.6 40.6-103.9 96.3-103.9 50.4 0 91.9 38.8 96.2 88 .4 4.4 2.4 8.5 5.7 11.4l30.7 27.2c3.5 3.1 9 1.2 9.9-3.4 2.2-11.8 3-24.2 2.1-36.8-4.9-72-63.3-130-135.4-134.4-82.7-5.1-151.8 59.5-154 140.6-2.1 78.9 62.6 147 141.6 148.7 33 .7 63.6-9.6 88.3-27.6L495 509.4c6.6 5.8 17 1.2 17-7.7l0-492c0-5.4-4.4-9.7-9.7-9.7L256 0z";
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
    exports.faAlgolia = exports.definition;
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

// src/_entries/faAlgolia.js
var import_faAlgolia = __toESM(require_faAlgolia());
var export_faAlgolia = import_faAlgolia.definition;
export {
  export_faAlgolia as faAlgolia
};
