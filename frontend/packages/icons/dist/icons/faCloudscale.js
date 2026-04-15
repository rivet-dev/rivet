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

// src/node_modules/@fortawesome/free-brands-svg-icons/faCloudscale.js
var require_faCloudscale = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faCloudscale.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "cloudscale";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f383";
    var svgPathData = "M318.1 154l-9.4 7.6c-22.5-19.3-51.5-33.6-83.3-33.6-71.6 0-129.4 60.8-129.4 132.3 0 6.6 .4 13.1 1.4 19.4-2-56 41.8-97.4 92.6-97.4 24.2 0 46.2 9.4 62.6 24.7l-25.2 20.4c-8.3-.9-16.8 1.8-23.1 8.1-11.1 11-11.1 28.9 0 40 11.1 11 28.9 11 40 0 6.3-6.3 9-14.9 8.1-23.1l75.2-88.8c6.3-6.5-3.3-15.9-9.5-9.6zM213.7 246.5a14 14 0 1 1 20.6 19 14 14 0 1 1 -20.6-19zM224 32a224 224 0 1 0 0 448 224 224 0 1 0 0-448zm0 64a160 160 0 1 1 0 320 160 160 0 1 1 0-320z";
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
    exports.faCloudscale = exports.definition;
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

// src/_entries/faCloudscale.js
var import_faCloudscale = __toESM(require_faCloudscale());
var export_faCloudscale = import_faCloudscale.definition;
export {
  export_faCloudscale as faCloudscale
};
