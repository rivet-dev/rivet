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

// src/node_modules/@fortawesome/free-brands-svg-icons/faCentos.js
var require_faCentos = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faCentos.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "centos";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f789";
    var svgPathData = "M289.6 97.5l31.6 31.7-76.3 76.5 0-108.2 44.7 0zM127.2 129.2l76.3 76.5 0-108.2-44.7 0-31.6 31.7zm41.5-41.6l44.7 0 0 127.9 10.8 10.8 10.8-10.8 0-127.9 44.7 0-55.5-55.6-55.5 55.6zm26.2 168.1l-10.8-10.8-128.6 0 0-44.8-55.5 55.6 55.5 55.6 0-44.8 128.6 0 10.8-10.8zM274.2 235l107.9 0 0-44.8-31.6-31.7-76.3 76.5zm173.3 20.7l-55.5-55.6 0 44.8-127.7 0-10.8 10.8 10.8 10.8 127.7 0 0 44.8 55.5-55.6zM65.4 176.2l32.5-31.7 90.3 90.5 15.3 0 0-15.3-90.3-90.5 31.6-31.7-79.4 0 0 78.7zM382.1 97.5l-78.5 0 31.6 31.7-90.3 90.5 0 15.3 15.3 0 90.3-90.5 31.6 31.7 0-78.7zM203.5 413.9l0-108.1-76.3 76.5 31.6 31.7 44.7 0 0-.1zM65.4 235l108.8 0-76.3-76.5-32.5 31.7 0 44.8zM382.1 335.2l-31.6 31.7-90.3-90.5-15.3 0 0 15.3 90.3 90.5-31.6 31.7 78.5 0 0-78.7zm0-58.8l-107.9 0 76.3 76.5 31.6-31.7 0-44.8zM321.2 382.2l-76.3-76.5 0 108.1 44.7 0 31.6-31.6zM97.9 352.9l76.3-76.5-108.8 0 0 44.8 32.5 31.7zm181.8 70.9l-44.7 0 0-127.9-10.8-10.8-10.8 10.8 0 127.9-44.7 0 55.5 55.6 55.5-55.6zM113.2 382.2l90.3-90.5 0-15.3-15.3 0-90.3 90.5-32.5-31.7 0 78.7 79.4 0-31.6-31.7z";
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
    exports.faCentos = exports.definition;
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

// src/_entries/faCentos.js
var import_faCentos = __toESM(require_faCentos());
var export_faCentos = import_faCentos.definition;
export {
  export_faCentos as faCentos
};
