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

// src/node_modules/@fortawesome/free-brands-svg-icons/faRenren.js
var require_faRenren = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faRenren.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "renren";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f18b";
    var svgPathData = "M214 169.1C214 279.5 153 374.5 66.4 416.5 30 373.2 8 317.7 8 256.6 8 133.9 97.1 32.2 214 12.5l0 156.6zM255 504c-42.9 0-83.3-11-118.5-30.4 57.2-36.1 103.4-90.7 118.5-154.6 15.5 63.9 61.7 118.5 118.8 154.7-35.1 19.3-75.5 30.3-118.8 30.3zm190.6-87.5C359 374.5 298 279.6 298 169.1l0-156.6c116.9 19.7 206 121.4 206 244.1 0 61.1-22 116.6-58.4 159.9z";
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
    exports.faRenren = exports.definition;
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

// src/_entries/faRenren.js
var import_faRenren = __toESM(require_faRenren());
var export_faRenren = import_faRenren.definition;
export {
  export_faRenren as faRenren
};
