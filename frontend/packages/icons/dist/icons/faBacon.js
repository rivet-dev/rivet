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

// src/node_modules/@fortawesome/free-solid-svg-icons/faBacon.js
var require_faBacon = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faBacon.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bacon";
    var width = 576;
    var height = 512;
    var aliases = [129363];
    var unicode = "f7e5";
    var svgPathData = "M557 96.7c14.4 14.4 12.4 38.4-4.3 50.2l-64.6 45.7c-43.7 30.9-79.2 71.9-103.4 119.6l-25.3 49.8c-25.1 49.3-62.1 91.5-107.8 122.6l-74.1 50.6c-13.1 8.9-30.7 7.3-41.8-3.9l-44.9-44.9 86.5-66.5c42.3-32.5 76.7-74.3 100.6-122l24.5-49.1c24.5-49 61.8-90.6 107.9-120.2l108.7-69.9 38 38zM484.2 23.9L384.3 88.2c-53.4 34.3-96.5 82.4-124.9 139.1l-24.5 49.1c-20.6 41.3-50.3 77.3-86.9 105.4l-91.4 70.3-36.9-36.9c-14.4-14.4-12.4-38.4 4.3-50.2l64.6-45.7c43.7-30.9 79.2-71.9 103.4-119.6l25.3-49.8C242.3 100.8 279.3 58.6 325 27.4l74.1-50.6c13.1-8.9 30.6-7.3 41.8 3.9l43.3 43.3z";
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
    exports.faBacon = exports.definition;
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

// src/_entries/faBacon.js
var import_faBacon = __toESM(require_faBacon());
var export_faBacon = import_faBacon.definition;
export {
  export_faBacon as faBacon
};
