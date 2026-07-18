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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTreeDeciduous.js
var require_faTreeDeciduous = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTreeDeciduous.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tree-deciduous";
    var width = 512;
    var height = 512;
    var aliases = [127795, "tree-alt"];
    var unicode = "f400";
    var svgPathData = "M384 80c-11.5 0-22.5 2.4-32.4 6.8C347 38.1 305.9 0 256 0s-91 38.1-95.6 86.8C150.5 82.4 139.5 80 128 80c-44.2 0-80 35.8-80 80c0 14.4 3.8 27.9 10.5 39.6C24.1 214.2 0 248.3 0 288c0 53 43 96 96 96l128 0 0 96c0 17.7 14.3 32 32 32s32-14.3 32-32l0-96 128 0c53 0 96-43 96-96c0-39.7-24.1-73.8-58.5-88.4c6.7-11.7 10.5-25.2 10.5-39.6c0-44.2-35.8-80-80-80z";
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
    exports.faTreeDeciduous = exports.definition;
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

// src/_entries/faTreeAlt.js
var import_faTreeDeciduous = __toESM(require_faTreeDeciduous());
var export_faTreeAlt = import_faTreeDeciduous.definition;
export {
  export_faTreeAlt as faTreeAlt
};
