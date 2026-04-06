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

// src/node_modules/@fortawesome/free-brands-svg-icons/faPadlet.js
var require_faPadlet = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faPadlet.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "padlet";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e4a0";
    var svgPathData = "M297.9 0l.1 0c7.6 .1 14.4 4.7 17.5 11.8l132 308.5 .3-.1 .2 .4-2.8 10-42.9 158c-3.7 16.2-19.7 26.3-35.8 22.6l-68.4-15.6-68.5 15.6c-16.1 3.7-32.1-6.4-35.8-22.6l-42.9-158-2.7-10 .1-.4 132.1-308.4c3-7 9.9-11.6 17.5-11.8zM160.1 322.1l131 39.1 6.9 122.5 7.9-121.5 130.6-39.3 .2-.1-131 25.1-8.6-320.2-5.2 320.2-131.8-25.8zM426 222.6l94.4-41 73.8 0-157 247.6 31.6-109-42.8-97.6zm171.5-41.2l41.4 76.2c4 7.5-3.9 15.9-11.6 12.2l-47.6-22.7 17.8-65.7zM127.3 318.5L158.7 430 1.6 154.5c-5.9-10.4 5.5-22 15.9-16.2l151.8 84.2-42.1 96z";
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
    exports.faPadlet = exports.definition;
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

// src/_entries/faPadlet.js
var import_faPadlet = __toESM(require_faPadlet());
var export_faPadlet = import_faPadlet.definition;
export {
  export_faPadlet as faPadlet
};
