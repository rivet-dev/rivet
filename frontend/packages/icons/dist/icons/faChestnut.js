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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faChestnut.js
var require_faChestnut = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faChestnut.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "chestnut";
    var width = 448;
    var height = 512;
    var aliases = [127792];
    var unicode = "e3f6";
    var svgPathData = "M224 32c5.3 0 10.3 2.7 13.3 7.1c2.9 4.3 7.9 9.7 15.5 16.6c7.5 6.8 16.7 14.3 27.2 22.8l3.1 2.5s0 0 0 0c20.3 16.4 44.7 36.1 68.2 59.6C400.8 190.2 448 258 448 352L0 352c0-94 47.2-161.8 96.7-211.3c23.5-23.5 47.9-43.2 68.2-59.6l3.1-2.5c10.6-8.5 19.7-16 27.2-22.8c7.6-6.9 12.7-12.3 15.5-16.6c3-4.5 8-7.1 13.3-7.1zM4 384L444 384c-14.2 55.2-64.3 96-124 96l-192 0C68.4 480 18.2 439.2 4 384z";
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
    exports.faChestnut = exports.definition;
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

// src/_entries/faChestnut.js
var import_faChestnut = __toESM(require_faChestnut());
var export_faChestnut = import_faChestnut.definition;
export {
  export_faChestnut as faChestnut
};
