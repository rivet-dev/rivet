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

// src/node_modules/@fortawesome/free-brands-svg-icons/faCloudsmith.js
var require_faCloudsmith = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faCloudsmith.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "cloudsmith";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f384";
    var svgPathData = "M512 227.6L512 284.5 284.4 512 227.6 512 0 284.4 0 227.6 227.6 0 284.5 0 512 227.6zm-256 162c17.8 .5 35.6-2.6 52.2-9.1s31.8-16.2 44.6-28.7 23-27.3 29.9-43.8 10.5-34.1 10.5-52-3.6-35.5-10.5-52-17.1-31.3-29.9-43.8-28-22.2-44.6-28.7-34.4-9.6-52.2-9.1c-17.8-.5-35.6 2.6-52.2 9.1s-31.8 16.3-44.6 28.7-23 27.3-29.9 43.8-10.5 34.1-10.5 52 3.6 35.5 10.5 52 17.1 31.3 29.9 43.8 28 22.2 44.6 28.7 34.4 9.6 52.2 9.1z";
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
    exports.faCloudsmith = exports.definition;
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

// src/_entries/faCloudsmith.js
var import_faCloudsmith = __toESM(require_faCloudsmith());
var export_faCloudsmith = import_faCloudsmith.definition;
export {
  export_faCloudsmith as faCloudsmith
};
